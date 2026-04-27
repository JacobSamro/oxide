// Live settings sourced from the same sqlite DB the admin UI writes to.
// We re-read on demand and broadcast changes to subscribers (TLS listener, S3 backend).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use rusqlite::Connection;
use serde::Deserialize;
use tokio::sync::Notify;
use tracing::{debug, warn};

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct DomainSettings {
    pub primary_domain: String,
    pub extra_domains: Vec<String>,
    pub public_url: String,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct SslSettings {
    pub enabled: bool,
    pub acme_email: String,
    pub staging: bool,
    pub http_redirect: bool,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct S3Settings {
    pub enabled: bool,
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    pub path_prefix: String,
    pub path_style: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LiveSettings {
    pub domain: DomainSettings,
    pub ssl: SslSettings,
    pub s3: S3Settings,
}

/// Wraps an Arc<LiveSettings> behind ArcSwap so handlers can read lock-free,
/// and a Notify so listeners (TLS, S3) can react to writes.
pub struct SettingsStore {
    db_path: PathBuf,
    state: ArcSwap<LiveSettings>,
    pub changed: Notify,
}

impl SettingsStore {
    pub fn open(db_path: impl AsRef<Path>) -> Result<Arc<Self>> {
        let db_path = db_path.as_ref().to_path_buf();
        let initial = read_all(&db_path).unwrap_or_default();
        let store = Arc::new(Self {
            db_path,
            state: ArcSwap::from_pointee(initial),
            changed: Notify::new(),
        });
        Ok(store)
    }

    pub fn snapshot(&self) -> Arc<LiveSettings> { self.state.load_full() }

    /// Re-read the DB; if changed, swap and notify.
    pub fn reload(&self) -> Result<bool> {
        let next = read_all(&self.db_path)?;
        let cur = self.state.load();
        if **cur != next {
            self.state.store(Arc::new(next));
            self.changed.notify_waiters();
            debug!("settings reloaded");
            Ok(true)
        } else { Ok(false) }
    }

    /// Background poll. Cheap query — sqlite handles this well.
    pub fn spawn_poll(self: &Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if let Err(e) = this.reload() {
                    warn!(?e, "settings reload failed");
                }
            }
        });
    }
}

fn read_all(path: &Path) -> Result<LiveSettings> {
    if !path.exists() {
        return Ok(LiveSettings::default());
    }
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("opening sqlite at {path:?}"))?;
    let domain = read_one::<DomainSettings>(&conn, "domain").unwrap_or_default();
    let ssl = read_one::<SslSettings>(&conn, "ssl").unwrap_or_default();
    let s3 = read_one::<S3Settings>(&conn, "s3").unwrap_or_default();

    // The admin UI uses camelCase keys; the structs above use snake_case via serde rename.
    Ok(LiveSettings { domain, ssl, s3 })
}

fn read_one<T: for<'de> Deserialize<'de> + Default>(conn: &Connection, key: &str) -> Result<T> {
    // Setting table may not exist yet on a fresh install — tolerate that.
    let mut stmt = match conn.prepare("SELECT value FROM Setting WHERE key = ?1") {
        Ok(s) => s,
        Err(_) => return Ok(T::default()),
    };
    let row: Result<String, _> = stmt.query_row([key], |r| r.get(0));
    match row {
        Ok(s) => Ok(serde_json::from_str(&camel_to_snake_json(&s))?),
        Err(_) => Ok(T::default()),
    }
}

/// The admin UI persists JSON with camelCase fields; the Rust structs rename via serde,
/// but we don't have the rename derive on every struct — translate field names here.
fn camel_to_snake_json(s: &str) -> String {
    // Cheap & predictable: only the small known set of keys are translated.
    s.replace("\"primaryDomain\"", "\"primary_domain\"")
     .replace("\"extraDomains\"", "\"extra_domains\"")
     .replace("\"publicUrl\"", "\"public_url\"")
     .replace("\"acmeEmail\"", "\"acme_email\"")
     .replace("\"httpRedirect\"", "\"http_redirect\"")
     .replace("\"accessKey\"", "\"access_key\"")
     .replace("\"secretKey\"", "\"secret_key\"")
     .replace("\"pathPrefix\"", "\"path_prefix\"")
     .replace("\"pathStyle\"", "\"path_style\"")
}
