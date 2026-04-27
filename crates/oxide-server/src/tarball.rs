use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Result};
use bytes::Bytes;
use dashmap::DashMap;
use futures::Stream;
use futures_util::stream::{self, StreamExt};
use reqwest::header::USER_AGENT;
use reqwest::StatusCode;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, Notify};
use tracing::warn;

use crate::config::TarballCacheConfig;
use crate::metrics::METRICS;
use crate::s3backend::S3Backend;
use crate::settings::SettingsStore;
use crate::storage;
use crate::upstream::Upstream;

const UA: &str = concat!("oxide/", env!("CARGO_PKG_VERSION"));

pub struct TarballCache {
    cfg: TarballCacheConfig,
    upstream: Upstream,
    settings: Arc<SettingsStore>,
    /// Active downloads: chunk-streamed to all subscribers as bytes arrive.
    inflight: Arc<DashMap<String, broadcast::Sender<Result<Bytes, String>>>>,
    /// Notifies when a key finishes (success or fail).
    finished: Arc<DashMap<String, Arc<Notify>>>,
}

impl TarballCache {
    pub fn new(cfg: TarballCacheConfig, upstream: Upstream, settings: Arc<SettingsStore>) -> Self {
        Self {
            cfg, upstream, settings,
            inflight: Arc::new(DashMap::new()),
            finished: Arc::new(DashMap::new()),
        }
    }

    fn s3(&self) -> Option<S3Backend> {
        let s = self.settings.snapshot();
        if !s.s3.enabled { return None; }
        match S3Backend::from_settings(&s.s3) {
            Ok(b) => Some(b),
            Err(e) => { tracing::warn!(?e, "s3 backend disabled (config error)"); None }
        }
    }

    pub fn local_path(&self, package: &str, file: &str) -> PathBuf {
        let mut p = self.cfg.path.clone();
        let safe_pkg = package.replace('/', "_2F_");
        p.push(safe_pkg);
        p.push(file);
        p
    }

    /// Fetch tarball: returns either a stream (live) or a fully-cached body.
    /// Returned tuple: (status, content-type, content-length-if-known, stream).
    pub async fn fetch(
        self: &Arc<Self>,
        package: &str,
        file: &str,
    ) -> Result<TarballResponse> {
        let path = self.local_path(package, file);

        // Cached on disk: stream from disk.
        if self.cfg.enabled {
            if let Ok(meta) = fs::metadata(&path).await {
                if meta.is_file() {
                    METRICS.tarball_cache.with_label_values(&["hit"]).inc();
                    let f = fs::File::open(&path).await?;
                    let stream = tokio_util::io::ReaderStream::new(f)
                        .map(|r| r.map_err(|e| e.to_string()));
                    return Ok(TarballResponse {
                        status: StatusCode::OK,
                        content_type: Some("application/octet-stream".into()),
                        content_length: Some(meta.len()),
                        body: Box::pin(stream),
                    });
                }
            }
        }

        // Cached in S3: stream from S3.
        if let Some(s3) = self.s3() {
            match s3.get_stream(package, file).await {
                Ok(Some((len, st))) => {
                    METRICS.tarball_cache.with_label_values(&["s3_hit"]).inc();
                    let mapped = st.map(|r| r.map_err(|e| e.to_string()));
                    return Ok(TarballResponse {
                        status: StatusCode::OK,
                        content_type: Some("application/octet-stream".into()),
                        content_length: if len > 0 { Some(len) } else { None },
                        body: Box::pin(mapped),
                    });
                }
                Ok(None) => {} // miss; fall through to upstream
                Err(e) => tracing::warn!(?e, "s3 get failed; falling back to upstream"),
            }
        }

        METRICS.tarball_cache.with_label_values(&["miss"]).inc();
        let key = format!("{package}/{file}");

        // Coalesce: subscribe to existing download if any.
        if let Some(tx) = self.inflight.get(&key).map(|v| v.clone()) {
            METRICS.coalesced.with_label_values(&["tarball"]).inc();
            let rx = tx.subscribe();
            let stream = broadcast_to_stream(rx);
            return Ok(TarballResponse {
                status: StatusCode::OK,
                content_type: Some("application/octet-stream".into()),
                content_length: None,
                body: Box::pin(stream),
            });
        }

        self.start_download(package, file, &key, &path).await
    }

    async fn start_download(
        self: &Arc<Self>,
        package: &str,
        file: &str,
        key: &str,
        path: &std::path::Path,
    ) -> Result<TarballResponse> {
        let (tx, _rx0) = broadcast::channel::<Result<Bytes, String>>(64);
        self.inflight.insert(key.to_string(), tx.clone());
        let notify = Arc::new(Notify::new());
        self.finished.insert(key.to_string(), notify.clone());

        let url = self.upstream.tarball_url(package, file);
        let permit = self.upstream.tar_sem.clone().acquire_owned().await?;
        METRICS.active_tarball_streams.with_label_values(&[&self.upstream.name]).inc();
        let started = Instant::now();

        let res = self.upstream.client.get(&url)
            .header(USER_AGENT, UA)
            .send().await;

        let res = match res {
            Ok(r) => r,
            Err(e) => {
                METRICS.active_tarball_streams.with_label_values(&[&self.upstream.name]).dec();
                self.inflight.remove(key);
                self.finished.remove(key);
                drop(permit);
                return Err(e.into());
            }
        };

        let status = res.status();
        METRICS.upstream_requests.with_label_values(&["tarball", status.as_str()]).inc();
        if !status.is_success() {
            METRICS.active_tarball_streams.with_label_values(&[&self.upstream.name]).dec();
            self.inflight.remove(key);
            self.finished.remove(key);
            return Err(anyhow!("upstream tarball status {status}"));
        }

        let content_type = res.headers().get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()).map(str::to_string);
        let content_length = res.content_length();

        // Caller subscribes for the live stream; the downloader is the producer.
        let rx = tx.subscribe();
        let live = broadcast_to_stream(rx);

        let this = self.clone();
        let key_owned = key.to_string();
        let path_owned = path.to_path_buf();
        let upstream_name = self.upstream.name.clone();
        let pkg_owned = package.to_string();
        let file_owned = file.to_string();
        tokio::spawn(async move {
            let mut tmp_path = path_owned.clone();
            let _ = tokio::fs::create_dir_all(tmp_path.parent().unwrap_or(std::path::Path::new("."))).await;
            tmp_path = storage::tmp_path(&path_owned);

            let mut total: u64 = 0;
            let mut writer = match tokio::fs::File::create(&tmp_path).await {
                Ok(f) => Some(f),
                Err(e) => {
                    warn!(?e, "tarball tmp create failed");
                    None
                }
            };
            // Buffer bytes for S3 upload only when an S3 backend is active.
            let s3 = this.s3();
            let mut s3_buf: Option<Vec<u8>> = if s3.is_some() { Some(Vec::new()) } else { None };

            let mut stream = res.bytes_stream();
            let mut ok = true;
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(b) => {
                        total += b.len() as u64;
                        if let Some(f) = writer.as_mut() {
                            if let Err(e) = f.write_all(&b).await {
                                warn!(?e, "tarball write failed; continuing without disk cache");
                                writer = None;
                            }
                        }
                        if let Some(buf) = s3_buf.as_mut() { buf.extend_from_slice(&b); }
                        let _ = tx.send(Ok(b));
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        ok = false;
                        break;
                    }
                }
            }

            if let Some(mut f) = writer.take() {
                let _ = f.flush().await;
                let _ = f.sync_all().await;
                drop(f);
                if ok {
                    if let Err(e) = tokio::fs::rename(&tmp_path, &path_owned).await {
                        warn!(?e, "tarball rename failed");
                    }
                } else {
                    let _ = tokio::fs::remove_file(&tmp_path).await;
                }
            }

            if ok {
                if let (Some(s3), Some(buf)) = (s3, s3_buf) {
                    if let Err(e) = s3.put(&pkg_owned, &file_owned, Bytes::from(buf)).await {
                        warn!(?e, "s3 put failed");
                    }
                }
            }

            METRICS.upstream_latency.with_label_values(&["tarball"]).observe(started.elapsed().as_secs_f64());
            METRICS.response_bytes.with_label_values(&["tarball"]).observe(total as f64);
            METRICS.active_tarball_streams.with_label_values(&[&upstream_name]).dec();
            this.inflight.remove(&key_owned);
            if let Some((_, n)) = this.finished.remove(&key_owned) { n.notify_waiters(); }
            drop(permit);
        });

        Ok(TarballResponse {
            status: StatusCode::OK,
            content_type,
            content_length,
            body: Box::pin(live),
        })
    }
}

pub struct TarballResponse {
    pub status: StatusCode,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub body: std::pin::Pin<Box<dyn Stream<Item = Result<Bytes, String>> + Send>>,
}

fn broadcast_to_stream(
    rx: broadcast::Receiver<Result<Bytes, String>>,
) -> impl Stream<Item = Result<Bytes, String>> + Send {
    stream::unfold((rx, false), |(mut rx, done)| async move {
        if done { return None; }
        match rx.recv().await {
            Ok(item) => {
                let stop = item.is_err();
                Some((item, (rx, stop)))
            }
            Err(broadcast::error::RecvError::Closed) => None,
            Err(broadcast::error::RecvError::Lagged(_)) => {
                Some((Err("client lagged broadcast".into()), (rx, true)))
            }
        }
    })
}
