use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::config::LogConfig;

pub fn init(cfg: &LogConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("oxide_server={lvl},tower_http=info,info", lvl = cfg.level)));

    let registry = tracing_subscriber::registry().with(filter);
    if cfg.json {
        registry.with(fmt::layer().json().with_target(true)).init();
    } else {
        registry.with(fmt::layer().with_target(false)).init();
    }
}
