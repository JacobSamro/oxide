use std::sync::Arc;

use anyhow::Result;
use tracing::warn;

use crate::config::Config;
use crate::db::Db;
use crate::local::LocalStore;
use crate::metadata::MetadataCache;
use crate::settings::SettingsStore;
use crate::storage;
use crate::tarball::TarballCache;
use crate::upstream::Upstream;

pub struct AppState {
    pub cfg: Config,
    pub settings: Arc<SettingsStore>,
    pub metadata: Arc<MetadataCache>,
    pub tarballs: Arc<TarballCache>,
    /// Writable DB handle, used for publish/auth. Optional so the proxy still runs in a
    /// pure-cache mode if the file isn't writable.
    pub db: Option<Db>,
    pub local: Option<Arc<LocalStore>>,
}

impl AppState {
    pub async fn new(cfg: Config) -> Result<Self> {
        let (uplink_name, up_cfg) = cfg.primary_uplink();
        let upstream = Upstream::new(uplink_name, up_cfg)?;

        if cfg.cache.metadata.disk_enabled {
            storage::ensure_dir(&cfg.cache.metadata.disk_path).await?;
        }
        if cfg.cache.tarballs.enabled && cfg.cache.tarballs.backend == "filesystem" {
            storage::ensure_dir(&cfg.cache.tarballs.path).await?;
        }

        let settings = SettingsStore::open(&cfg.server.db_path)?;
        settings.spawn_poll();

        let public_url = {
            let s = settings.snapshot();
            if !s.domain.public_url.is_empty() {
                s.domain.public_url.clone()
            } else {
                cfg.server.public_url.clone()
            }
        };

        // Open the writable DB. If the file isn't writable yet (the Bun side hasn't applied
        // the schema), publish stays disabled until the next restart — log and continue.
        let db = match Db::open(&cfg.server.db_path) {
            Ok(d) => Some(d),
            Err(e) => { warn!(?e, "publish disabled: cannot open writable db"); None }
        };

        let local = if db.is_some() {
            storage::ensure_dir(&cfg.server.local_storage_path).await?;
            Some(Arc::new(LocalStore::new(&cfg.server.local_storage_path)))
        } else { None };

        let metadata = Arc::new(MetadataCache::new(
            cfg.cache.metadata.clone(),
            public_url,
            upstream.clone(),
            up_cfg.metadata_ttl,
            up_cfg.stale_while_revalidate,
        ));

        let tarballs = Arc::new(TarballCache::new(
            cfg.cache.tarballs.clone(),
            upstream.clone(),
            settings.clone(),
        ));

        Ok(Self { cfg, settings, metadata, tarballs, db, local })
    }
}
