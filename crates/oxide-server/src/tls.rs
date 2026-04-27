// HTTPS via Let's Encrypt using rustls-acme. The ACME state drives a cert resolver
// that we hand to a rustls::ServerConfig used by axum-server.
//
// rustls-acme handles the ACME order/challenge dance via TLS-ALPN-01 on :443, so port 80
// is not required for issuance. We still serve HTTP on :80 for redirect-to-HTTPS.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use futures_util::StreamExt;
use rustls::ServerConfig;
use rustls_acme::caches::DirCache;
use rustls_acme::AcmeConfig;
use tracing::{info, warn};

use crate::settings::SslSettings;

const LE_PROD: &str = "https://acme-v02.api.letsencrypt.org/directory";
const LE_STAGING: &str = "https://acme-staging-v02.api.letsencrypt.org/directory";

pub async fn build_acme_server_config(
    domains: Vec<String>,
    ssl: &SslSettings,
    cache_dir: PathBuf,
) -> Result<Arc<ServerConfig>> {
    anyhow::ensure!(!domains.is_empty(), "ssl enabled but no domains configured");
    anyhow::ensure!(!ssl.acme_email.is_empty(), "ssl contact email required");
    tokio::fs::create_dir_all(&cache_dir).await.ok();

    let mut state = AcmeConfig::new(domains.clone())
        .contact(vec![format!("mailto:{}", ssl.acme_email)])
        .cache(DirCache::new(cache_dir))
        .directory(if ssl.staging { LE_STAGING } else { LE_PROD })
        .state();

    let resolver = state.resolver();

    // Drive ACME events in the background — this is what triggers cert issuance + renewal.
    tokio::spawn(async move {
        loop {
            match state.next().await {
                Some(Ok(ok)) => info!(?ok, "acme event"),
                Some(Err(err)) => warn!(?err, "acme error"),
                None => break,
            }
        }
    });

    let mut cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(resolver);
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"acme-tls/1".to_vec()];
    Ok(Arc::new(cfg))
}

pub async fn serve_https(
    app: Router,
    server_config: Arc<ServerConfig>,
    https_addr: std::net::SocketAddr,
) -> Result<()> {
    info!("binding TLS on {https_addr}");
    let cfg = RustlsConfig::from_config(server_config);
    axum_server::bind_rustls(https_addr, cfg)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

/// Plain HTTP server. Used when SSL is disabled, or as the :80 listener that 301s to HTTPS.
pub async fn serve_http(app: Router, addr: std::net::SocketAddr) -> Result<()> {
    info!("binding HTTP on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

/// A tiny app that 301s every request to https://<host><path>.
pub fn redirect_app() -> Router {
    use axum::http::{header, StatusCode, Uri};
    use axum::response::{IntoResponse, Response};
    Router::new().fallback(|uri: Uri, host: axum::extract::Host| async move {
        let path = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
        let target = format!("https://{}{}", host.0, path);
        Response::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header(header::LOCATION, target)
            .body(axum::body::Body::empty())
            .unwrap()
            .into_response()
    })
}
