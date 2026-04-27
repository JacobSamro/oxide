use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub log: LogConfig,
    pub uplinks: HashMap<String, UplinkConfig>,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub audit: AuditConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_http_listen")]
    pub http_listen: String,
    #[serde(default = "default_https_listen")]
    pub https_listen: String,
    /// Public base URL fallback used when the runtime "domain" setting is empty.
    #[serde(default = "default_public_url")]
    pub public_url: String,
    /// Where the admin sqlite DB lives. Same file the Bun/Nuxt UI writes to.
    #[serde(default = "default_db_path")]
    pub db_path: PathBuf,
    /// ACME cache dir — issued certs are persisted here.
    #[serde(default = "default_acme_cache")]
    pub acme_cache_dir: PathBuf,
    /// Where locally-published package tarballs live. Distinct from the upstream cache.
    #[serde(default = "default_local_storage")]
    pub local_storage_path: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            http_listen: default_http_listen(),
            https_listen: default_https_listen(),
            public_url: default_public_url(),
            db_path: default_db_path(),
            acme_cache_dir: default_acme_cache(),
            local_storage_path: default_local_storage(),
        }
    }
}

fn default_http_listen() -> String { "0.0.0.0:80".into() }
fn default_https_listen() -> String { "0.0.0.0:443".into() }
fn default_public_url() -> String { "http://localhost:4873".into() }
fn default_db_path() -> PathBuf { PathBuf::from("./data/oxide.db") }
fn default_acme_cache() -> PathBuf { PathBuf::from("./data/acme") }
fn default_local_storage() -> PathBuf { PathBuf::from("./data/local") }

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub json: bool,
}

fn default_log_level() -> String { "info".into() }

#[derive(Debug, Clone, Deserialize)]
pub struct UplinkConfig {
    pub url: String,
    #[serde(with = "humantime_serde", default = "ttl_default")]
    pub metadata_ttl: Duration,
    #[serde(with = "humantime_serde", default = "swr_default")]
    pub stale_while_revalidate: Duration,
    #[serde(with = "humantime_serde", default = "timeout_default")]
    pub timeout: Duration,
    #[serde(default = "max_conn_default")]
    pub max_connections: usize,
    #[serde(default = "max_meta_default")]
    pub max_concurrent_metadata_fetches: usize,
    #[serde(default = "max_tar_default")]
    pub max_concurrent_tarball_fetches: usize,
    #[serde(default)]
    pub retry: RetryConfig,
}

fn ttl_default() -> Duration { Duration::from_secs(7 * 24 * 3600) }
fn swr_default() -> Duration { Duration::from_secs(24 * 3600) }
fn timeout_default() -> Duration { Duration::from_secs(30) }
fn max_conn_default() -> usize { 200 }
fn max_meta_default() -> usize { 50 }
fn max_tar_default() -> usize { 100 }

#[derive(Debug, Clone, Deserialize)]
pub struct RetryConfig {
    #[serde(default = "retry_attempts")]
    pub attempts: u32,
    #[serde(default = "retry_honor")]
    pub honor_retry_after: bool,
}

impl Default for RetryConfig {
    fn default() -> Self { Self { attempts: 1, honor_retry_after: true } }
}

fn retry_attempts() -> u32 { 1 }
fn retry_honor() -> bool { true }

#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    #[serde(default)]
    pub metadata: MetadataCacheConfig,
    #[serde(default)]
    pub tarballs: TarballCacheConfig,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self { metadata: MetadataCacheConfig::default(), tarballs: TarballCacheConfig::default() }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetadataCacheConfig {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "default_meta_mem", deserialize_with = "deser_size")]
    pub memory_max_bytes: u64,
    #[serde(default = "yes")]
    pub disk_enabled: bool,
    #[serde(default = "default_meta_path")]
    pub disk_path: PathBuf,
    #[serde(default = "yes")]
    pub precompute_abbreviated: bool,
    #[serde(default = "yes")]
    pub precompress: bool,
}

impl Default for MetadataCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_max_bytes: default_meta_mem(),
            disk_enabled: true,
            disk_path: default_meta_path(),
            precompute_abbreviated: true,
            precompress: true,
        }
    }
}

fn default_meta_mem() -> u64 { 512 * 1024 * 1024 }
fn default_meta_path() -> PathBuf { PathBuf::from("./data/metadata") }

#[derive(Debug, Clone, Deserialize)]
pub struct TarballCacheConfig {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_tar_path")]
    pub path: PathBuf,
}

impl Default for TarballCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: default_backend(),
            path: default_tar_path(),
        }
    }
}

fn default_backend() -> String { "filesystem".into() }
fn default_tar_path() -> PathBuf { PathBuf::from("./data/tarballs") }

#[derive(Debug, Clone, Deserialize)]
pub struct AuditConfig {
    #[serde(default = "default_audit_mode")]
    pub mode: AuditMode,
}

impl Default for AuditConfig {
    fn default() -> Self { Self { mode: AuditMode::Disabled } }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditMode { Disabled, Proxy, Empty }

fn default_audit_mode() -> AuditMode { AuditMode::Disabled }

fn yes() -> bool { true }

fn deser_size<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    use serde::de::Error;
    let s = String::deserialize(d)?;
    parse_size::parse_size(&s).map_err(D::Error::custom)
}

pub fn load(path: &str) -> Result<Config> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading config {path}"))?;
    let cfg: Config = serde_yaml::from_str(&text).context("parsing config")?;
    Ok(cfg)
}

impl Config {
    /// First (and currently only) uplink. Multi-uplink can come later.
    pub fn primary_uplink(&self) -> (&str, &UplinkConfig) {
        self.uplinks.iter().next().map(|(k, v)| (k.as_str(), v))
            .expect("at least one uplink must be configured")
    }
}
