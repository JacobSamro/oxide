use std::sync::Arc;

use anyhow::Result;

use crate::config::Config;
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

        // Live settings from sqlite: domain → public_url override; ssl/s3 watched at runtime.
        let settings = SettingsStore::open(&cfg.server.db_path)?;
        settings.spawn_poll();

        // Pick the public URL: prefer the runtime "domain.publicUrl" if non-empty.
        let public_url = {
            let s = settings.snapshot();
            if !s.domain.public_url.is_empty() {
                s.domain.public_url.clone()
            } else {
                cfg.server.public_url.clone()
            }
        };

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

        Ok(Self { cfg, settings, metadata, tarballs })
    }
}
