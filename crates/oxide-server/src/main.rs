use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use clap::Parser;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

mod coalesce;
mod config;
mod logger;
mod metadata;
mod metrics;
mod routes;
mod s3backend;
mod settings;
mod state;
mod storage;
mod tarball;
mod tls;
mod transform;
mod upstream;

#[derive(Parser, Debug)]
#[command(name = "oxide", version, about = "Rust-Rite npm registry proxy")]
struct Cli {
    #[arg(short, long, env = "OXIDE_CONFIG", default_value = "oxide.yaml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::load(&cli.config)?;
    logger::init(&cfg.log);

    // Install rustls' default crypto provider before any TLS work.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let state = state::AppState::new(cfg.clone()).await?;
    let state = Arc::new(state);

    let app: Router = routes::router(state.clone()).layer(TraceLayer::new_for_http());

    let ssl = state.settings.snapshot().ssl.clone();
    let domain = state.settings.snapshot().domain.clone();
    let http_addr: SocketAddr = cfg.server.http_listen.parse()?;
    let https_addr: SocketAddr = cfg.server.https_listen.parse()?;

    if ssl.enabled && !domain.primary_domain.is_empty() {
        let mut domains = vec![domain.primary_domain.clone()];
        domains.extend(domain.extra_domains.iter().cloned());

        info!(?domains, staging = ssl.staging, "starting with Let's Encrypt TLS");
        let server_cfg = tls::build_acme_server_config(domains, &ssl, cfg.server.acme_cache_dir.clone()).await?;

        // Run HTTPS plus a small HTTP listener for redirect (or app fallback).
        let https = tokio::spawn({
            let app = app.clone();
            async move { tls::serve_https(app, server_cfg, https_addr).await }
        });
        let http_app = if ssl.http_redirect { tls::redirect_app() } else { app };
        let http = tokio::spawn(async move { tls::serve_http(http_app, http_addr).await });

        tokio::select! {
            r = https => if let Err(e) = r? { warn!(?e, "https terminated") },
            r = http  => if let Err(e) = r? { warn!(?e, "http terminated") },
        }
    } else {
        info!("SSL disabled — serving plain HTTP only");
        tls::serve_http(app, http_addr).await?;
    }
    Ok(())
}
