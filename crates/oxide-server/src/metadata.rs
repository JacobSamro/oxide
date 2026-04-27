use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use bytes::Bytes;
use moka::future::Cache;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::coalesce::Singleflight;
use crate::config::MetadataCacheConfig;
use crate::metrics::METRICS;
use crate::storage;
use crate::transform;
use crate::upstream::{fetch_metadata, Upstream};

/// One cached metadata entry, ready to serve.
///
/// `full` is the rewritten full document; `abbreviated` is the abbreviated variant.
/// Compressed copies are precomputed when the config allows it.
#[derive(Clone)]
pub struct CachedMetadata {
    pub full: Bytes,
    pub full_gzip: Option<Bytes>,
    pub full_br: Option<Bytes>,
    pub abbreviated: Option<Bytes>,
    pub abbreviated_gzip: Option<Bytes>,
    pub abbreviated_br: Option<Bytes>,
    pub etag: Option<String>,
    pub fetched_at: Instant,
    /// Hard expiry past which we won't serve at all (fetched_at + ttl + swr).
    pub hard_expiry: Instant,
    /// Soft expiry past which we trigger a background refresh (fetched_at + ttl).
    pub soft_expiry: Instant,
}

impl CachedMetadata {
    pub fn approx_size(&self) -> u32 {
        let mut s = self.full.len();
        s += self.full_gzip.as_ref().map(|b| b.len()).unwrap_or(0);
        s += self.full_br.as_ref().map(|b| b.len()).unwrap_or(0);
        s += self.abbreviated.as_ref().map(|b| b.len()).unwrap_or(0);
        s += self.abbreviated_gzip.as_ref().map(|b| b.len()).unwrap_or(0);
        s += self.abbreviated_br.as_ref().map(|b| b.len()).unwrap_or(0);
        s as u32
    }

    pub fn is_fresh(&self) -> bool { Instant::now() < self.soft_expiry }
    pub fn is_servable(&self) -> bool { Instant::now() < self.hard_expiry }
}

#[derive(Serialize, Deserialize)]
struct DiskRecord {
    full: Vec<u8>,
    etag: Option<String>,
    fetched_at_secs: u64,
}

pub struct MetadataCache {
    cfg: MetadataCacheConfig,
    public_url: String,
    mem: Cache<String, Arc<CachedMetadata>>,
    coalesce: Singleflight<String, FetchOutcome>,
    pub upstream: Upstream,
    pub ttl: Duration,
    pub swr: Duration,
}

#[derive(Clone)]
pub enum FetchOutcome {
    Ok(Arc<CachedMetadata>),
    Status(u16),
    Err(String),
}

impl MetadataCache {
    pub fn new(cfg: MetadataCacheConfig, public_url: String, upstream: Upstream, ttl: Duration, swr: Duration) -> Self {
        let max_bytes = cfg.memory_max_bytes;
        let mem = Cache::builder()
            .weigher(|_k: &String, v: &Arc<CachedMetadata>| v.approx_size())
            .max_capacity(max_bytes)
            .eviction_listener(|_k, _v, _cause| {
                METRICS.mem_cache_evictions.with_label_values(&["metadata"]).inc();
            })
            .build();
        Self {
            cfg, public_url, mem,
            coalesce: Singleflight::new(),
            upstream, ttl, swr,
        }
    }

    pub fn disk_path(&self, package: &str) -> PathBuf {
        let mut p = self.cfg.disk_path.clone();
        // shard by first 2 chars to avoid huge dirs; scoped pkg keeps slash.
        let safe = package.replace('/', "_2F_");
        let head: String = safe.chars().take(2).collect();
        p.push(head);
        p.push(format!("{safe}.json"));
        p
    }

    async fn load_from_disk(&self, package: &str) -> Option<Arc<CachedMetadata>> {
        if !self.cfg.disk_enabled { return None; }
        let path = self.disk_path(package);
        let bytes = storage::read_optional(&path).await.ok().flatten()?;
        let rec: DiskRecord = serde_json::from_slice(&bytes).ok()?;
        // Disk-loaded entries are treated as soft-expired; they must be revalidated soon.
        let fetched_at = Instant::now()
            .checked_sub(Duration::from_secs(rec.fetched_at_secs.min(60 * 60 * 24 * 30)))
            .unwrap_or_else(Instant::now);
        let cm = build_cached(&self.cfg, &self.public_url, Bytes::from(rec.full), rec.etag, fetched_at, self.ttl, self.swr).ok()?;
        Some(Arc::new(cm))
    }

    async fn write_disk(&self, package: &str, cm: &CachedMetadata) {
        if !self.cfg.disk_enabled { return; }
        let path = self.disk_path(package);
        let rec = DiskRecord {
            full: cm.full.to_vec(),
            etag: cm.etag.clone(),
            fetched_at_secs: cm.fetched_at.elapsed().as_secs(),
        };
        if let Ok(bytes) = serde_json::to_vec(&rec) {
            if let Err(e) = storage::write_atomic(&path, &bytes).await {
                warn!(?e, ?path, "metadata disk write failed");
            }
        }
    }

    /// Public entrypoint: returns a cached entry, fetching/coalescing as needed.
    pub async fn get(self: &Arc<Self>, package: &str) -> Result<Arc<CachedMetadata>> {
        if let Some(cm) = self.mem.get(package).await {
            if cm.is_fresh() {
                METRICS.meta_cache.with_label_values(&["hit"]).inc();
                return Ok(cm);
            }
            if cm.is_servable() {
                METRICS.meta_cache.with_label_values(&["swr"]).inc();
                self.spawn_refresh(package.to_string());
                return Ok(cm);
            }
        }

        // Try disk on memory miss.
        if let Some(cm) = self.load_from_disk(package).await {
            self.mem.insert(package.to_string(), cm.clone()).await;
            METRICS.meta_cache.with_label_values(&["disk_hit"]).inc();
            // Disk entry is treated as stale; refresh in background and serve immediately.
            self.spawn_refresh(package.to_string());
            return Ok(cm);
        }

        METRICS.meta_cache.with_label_values(&["miss"]).inc();
        self.fetch_and_store(package, None).await
    }

    fn spawn_refresh(self: &Arc<Self>, package: String) {
        let this = self.clone();
        tokio::spawn(async move {
            let if_none_match = this.mem.get(&package).await.and_then(|c| c.etag.clone());
            if let Err(e) = this.fetch_and_store(&package, if_none_match.as_deref()).await {
                warn!(%package, ?e, "background refresh failed");
            }
        });
    }

    async fn fetch_and_store(self: &Arc<Self>, package: &str, if_none_match: Option<&str>) -> Result<Arc<CachedMetadata>> {
        let key = package.to_string();
        let this = self.clone();
        let inm = if_none_match.map(str::to_string);
        let pkg = key.clone();
        let (out, coalesced) = self.coalesce.run(key.clone(), || async move {
            match fetch_metadata(&this.upstream, &pkg, "application/json", inm.as_deref()).await {
                Ok(res) => {
                    if res.status == StatusCode::NOT_MODIFIED {
                        // Existing entry is still valid: bump its expiries.
                        if let Some(existing) = this.mem.get(&pkg).await {
                            let bumped = bump_expiry(&existing, this.ttl, this.swr);
                            this.mem.insert(pkg.clone(), Arc::new(bumped.clone())).await;
                            return FetchOutcome::Ok(Arc::new(bumped));
                        }
                    }
                    if !res.status.is_success() {
                        if res.status == StatusCode::TOO_MANY_REQUESTS {
                            // Rate-limited: serve stale if available, propagate upstream.
                            return FetchOutcome::Status(res.status.as_u16());
                        }
                        return FetchOutcome::Status(res.status.as_u16());
                    }
                    let cm = match build_cached(&this.cfg, &this.public_url, res.body, res.etag, Instant::now(), this.ttl, this.swr) {
                        Ok(c) => c,
                        Err(e) => return FetchOutcome::Err(e.to_string()),
                    };
                    let arc = Arc::new(cm.clone());
                    this.mem.insert(pkg.clone(), arc.clone()).await;
                    METRICS.mem_cache_size.with_label_values(&["metadata"]).set(this.mem.weighted_size() as i64);
                    this.write_disk(&pkg, &cm).await;
                    FetchOutcome::Ok(arc)
                }
                Err(e) => FetchOutcome::Err(e.to_string()),
            }
        }).await;

        if coalesced { METRICS.coalesced.with_label_values(&["metadata"]).inc(); }

        match (*out).clone() {
            FetchOutcome::Ok(v) => Ok(v),
            FetchOutcome::Status(s) => {
                // Upstream non-2xx: serve stale if we still have it within hard expiry.
                if let Some(existing) = self.mem.get(package).await {
                    if existing.is_servable() {
                        METRICS.meta_cache.with_label_values(&["stale_hit"]).inc();
                        debug!(%package, status=s, "serving stale metadata after upstream non-2xx");
                        return Ok(existing);
                    }
                }
                anyhow::bail!("upstream status {s}")
            }
            FetchOutcome::Err(e) => {
                if let Some(existing) = self.mem.get(package).await {
                    if existing.is_servable() {
                        METRICS.meta_cache.with_label_values(&["stale_hit"]).inc();
                        return Ok(existing);
                    }
                }
                anyhow::bail!(e)
            }
        }
    }

    pub async fn invalidate(&self, package: &str) {
        self.mem.invalidate(package).await;
    }
}

fn bump_expiry(existing: &CachedMetadata, ttl: Duration, swr: Duration) -> CachedMetadata {
    let now = Instant::now();
    CachedMetadata {
        full: existing.full.clone(),
        full_gzip: existing.full_gzip.clone(),
        full_br: existing.full_br.clone(),
        abbreviated: existing.abbreviated.clone(),
        abbreviated_gzip: existing.abbreviated_gzip.clone(),
        abbreviated_br: existing.abbreviated_br.clone(),
        etag: existing.etag.clone(),
        fetched_at: now,
        soft_expiry: now + ttl,
        hard_expiry: now + ttl + swr,
    }
}

fn build_cached(
    cfg: &MetadataCacheConfig,
    public_url: &str,
    raw: Bytes,
    etag: Option<String>,
    fetched_at: Instant,
    ttl: Duration,
    swr: Duration,
) -> Result<CachedMetadata> {
    let rewritten = transform::rewrite_tarball_urls(&raw, public_url)?;
    let abbreviated = if cfg.precompute_abbreviated {
        Some(transform::abbreviate(&rewritten)?)
    } else { None };

    let (full_gz, full_br) = if cfg.precompress {
        (Some(transform::gzip(&rewritten)?), Some(transform::brotli_compress(&rewritten)?))
    } else { (None, None) };

    let (abbr_gz, abbr_br) = match (cfg.precompress, &abbreviated) {
        (true, Some(a)) => (Some(transform::gzip(a)?), Some(transform::brotli_compress(a)?)),
        _ => (None, None),
    };

    Ok(CachedMetadata {
        full: rewritten,
        full_gzip: full_gz,
        full_br,
        abbreviated,
        abbreviated_gzip: abbr_gz,
        abbreviated_br: abbr_br,
        etag,
        fetched_at,
        soft_expiry: fetched_at + ttl,
        hard_expiry: fetched_at + ttl + swr,
    })
}
