use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, IF_NONE_MATCH, USER_AGENT};
use reqwest::{Client, StatusCode};
use tokio::sync::Semaphore;
use tracing::{debug, warn};

use crate::config::UplinkConfig;
use crate::metrics::METRICS;

const UPSTREAM_UA: &str = concat!("oxide/", env!("CARGO_PKG_VERSION"));

#[derive(Clone)]
pub struct Upstream {
    pub name: String,
    pub url: String,
    pub client: Client,
    pub meta_sem: Arc<Semaphore>,
    pub tar_sem: Arc<Semaphore>,
    pub timeout: Duration,
}

impl Upstream {
    pub fn new(name: &str, cfg: &UplinkConfig) -> Result<Self> {
        let client = Client::builder()
            .pool_max_idle_per_host(cfg.max_connections)
            .timeout(cfg.timeout)
            .user_agent(UPSTREAM_UA)
            .build()?;
        Ok(Self {
            name: name.to_string(),
            url: cfg.url.trim_end_matches('/').to_string(),
            client,
            meta_sem: Arc::new(Semaphore::new(cfg.max_concurrent_metadata_fetches)),
            tar_sem: Arc::new(Semaphore::new(cfg.max_concurrent_tarball_fetches)),
            timeout: cfg.timeout,
        })
    }

    pub fn meta_url(&self, package: &str) -> String {
        // Scoped: keep the slash; npm clients commonly use percent-encoded form, support both.
        format!("{}/{}", self.url, package)
    }

    pub fn tarball_url(&self, package: &str, file: &str) -> String {
        format!("{}/{}/-/{}", self.url, package, file)
    }
}

pub struct MetadataFetch {
    pub status: StatusCode,
    pub body: Bytes,
    pub etag: Option<String>,
    pub content_type: Option<String>,
    pub retry_after: Option<Duration>,
}

pub async fn fetch_metadata(
    up: &Upstream,
    package: &str,
    accept: &str,
    if_none_match: Option<&str>,
) -> Result<MetadataFetch> {
    let _permit = up.meta_sem.clone().acquire_owned().await?;
    METRICS.active_meta_fetches.with_label_values(&[&up.name]).inc();
    let started = Instant::now();
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_str(accept)?);
    headers.insert(USER_AGENT, HeaderValue::from_static(UPSTREAM_UA));
    if let Some(etag) = if_none_match {
        if let Ok(v) = HeaderValue::from_str(etag) { headers.insert(IF_NONE_MATCH, v); }
    }

    let url = up.meta_url(package);
    debug!(%url, "metadata fetch");
    let res = up.client.get(&url).headers(headers).send().await;
    METRICS.active_meta_fetches.with_label_values(&[&up.name]).dec();
    let latency = started.elapsed().as_secs_f64();
    METRICS.upstream_latency.with_label_values(&["metadata"]).observe(latency);

    let res = res?;
    let status = res.status();
    METRICS.upstream_requests.with_label_values(&["metadata", status.as_str()]).inc();

    let etag = res.headers().get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok()).map(str::to_string);
    let content_type = res.headers().get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok()).map(str::to_string);
    let retry_after = parse_retry_after(res.headers().get(HeaderName::from_static("retry-after")));

    if status == StatusCode::TOO_MANY_REQUESTS {
        METRICS.rate_limited.with_label_values(&[&up.name]).inc();
        warn!(%url, ?retry_after, "upstream rate-limited");
    }

    let body = if status == StatusCode::NOT_MODIFIED { Bytes::new() } else { res.bytes().await? };
    Ok(MetadataFetch { status, body, etag, content_type, retry_after })
}

fn parse_retry_after(h: Option<&HeaderValue>) -> Option<Duration> {
    let v = h?.to_str().ok()?;
    if let Ok(secs) = v.parse::<u64>() { return Some(Duration::from_secs(secs)); }
    None
}
