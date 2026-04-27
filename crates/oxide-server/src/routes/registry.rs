use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use tracing::warn;

use crate::metadata::CachedMetadata;
use crate::metrics::METRICS;
use crate::state::AppState;

const ABBREVIATED: &str = "application/vnd.npm.install-v1+json";

pub async fn metadata_unscoped(
    State(state): State<Arc<AppState>>,
    Path(package): Path<String>,
    headers: HeaderMap,
) -> Response {
    serve_metadata(state, decode_pkg(&package), headers).await
}

pub async fn metadata_scoped(
    State(state): State<Arc<AppState>>,
    Path((scope, package)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let scope = if scope.starts_with('@') { scope } else { format!("@{scope}") };
    let pkg = format!("{scope}/{package}");
    serve_metadata(state, pkg, headers).await
}

pub async fn tarball_unscoped(
    State(state): State<Arc<AppState>>,
    Path((package, file)): Path<(String, String)>,
) -> Response {
    serve_tarball(state, decode_pkg(&package), file).await
}

pub async fn tarball_scoped(
    State(state): State<Arc<AppState>>,
    Path((scope, package, file)): Path<(String, String, String)>,
) -> Response {
    let scope = if scope.starts_with('@') { scope } else { format!("@{scope}") };
    let pkg = format!("{scope}/{package}");
    serve_tarball(state, pkg, file).await
}

pub async fn invalidate(
    State(state): State<Arc<AppState>>,
    Path(package): Path<String>,
) -> Response {
    state.metadata.invalidate(&decode_pkg(&package)).await;
    (StatusCode::NO_CONTENT, ()).into_response()
}

pub async fn reload(State(state): State<Arc<AppState>>) -> Response {
    match state.settings.reload() {
        Ok(true)  => (StatusCode::OK, "reloaded\n").into_response(),
        Ok(false) => (StatusCode::OK, "no change\n").into_response(),
        Err(e)    => (StatusCode::BAD_GATEWAY, format!("reload failed: {e}")).into_response(),
    }
}

fn decode_pkg(s: &str) -> String {
    // npm uses %2F in scoped names when sent as a single path segment.
    s.replace("%2f", "/").replace("%2F", "/")
}

async fn serve_metadata(state: Arc<AppState>, package: String, headers: HeaderMap) -> Response {
    let want_abbreviated = headers.get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains(ABBREVIATED))
        .unwrap_or(false);

    let cm = match state.metadata.get(&package).await {
        Ok(c) => c,
        Err(e) => {
            warn!(%package, ?e, "metadata fetch failed");
            return (StatusCode::BAD_GATEWAY, format!("upstream error: {e}")).into_response();
        }
    };

    let if_none_match = headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok());
    if let (Some(client_etag), Some(server_etag)) = (if_none_match, cm.etag.as_deref()) {
        if etag_matches(client_etag, server_etag) {
            return (StatusCode::NOT_MODIFIED, ()).into_response();
        }
    }

    let accept_enc = headers.get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok()).unwrap_or("");

    let (bytes, encoding, content_type) = pick_payload(&cm, want_abbreviated, accept_enc);
    METRICS.response_bytes.with_label_values(&["metadata"]).observe(bytes.len() as f64);

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "public, max-age=60")
        .header("x-oxide-cache", if cm.is_fresh() { "HIT" } else { "STALE" });
    if let Some(enc) = encoding {
        builder = builder.header(header::CONTENT_ENCODING, enc).header(header::VARY, "Accept-Encoding, Accept");
    } else {
        builder = builder.header(header::VARY, "Accept-Encoding, Accept");
    }
    if let Some(etag) = &cm.etag {
        if let Ok(v) = HeaderValue::from_str(etag) { builder = builder.header(header::ETAG, v); }
    }
    builder.body(Body::from(bytes)).unwrap()
}

fn etag_matches(client: &str, server: &str) -> bool {
    client.split(',').map(str::trim).any(|t| t == server || t == "*")
}

fn pick_payload(cm: &CachedMetadata, abbreviated: bool, accept_enc: &str) -> (Bytes, Option<&'static str>, &'static str) {
    let want_br = accept_enc.contains("br");
    let want_gz = accept_enc.contains("gzip");
    if abbreviated {
        if let Some(b) = &cm.abbreviated {
            if want_br { if let Some(z) = &cm.abbreviated_br { return (z.clone(), Some("br"), ABBREVIATED); } }
            if want_gz { if let Some(z) = &cm.abbreviated_gzip { return (z.clone(), Some("gzip"), ABBREVIATED); } }
            return (b.clone(), None, ABBREVIATED);
        }
    }
    if want_br { if let Some(z) = &cm.full_br { return (z.clone(), Some("br"), "application/json"); } }
    if want_gz { if let Some(z) = &cm.full_gzip { return (z.clone(), Some("gzip"), "application/json"); } }
    (cm.full.clone(), None, "application/json")
}

async fn serve_tarball(state: Arc<AppState>, package: String, file: String) -> Response {
    let res = match state.tarballs.fetch(&package, &file).await {
        Ok(r) => r,
        Err(e) => {
            warn!(%package, %file, ?e, "tarball fetch failed");
            return (StatusCode::BAD_GATEWAY, format!("tarball error: {e}")).into_response();
        }
    };

    let mut builder = Response::builder().status(res.status);
    if let Some(ct) = res.content_type {
        if let Ok(v) = HeaderValue::from_str(&ct) { builder = builder.header(header::CONTENT_TYPE, v); }
    }
    if let Some(len) = res.content_length {
        builder = builder.header(header::CONTENT_LENGTH, len);
    }

    use futures_util::StreamExt;
    let stream = res.body.map(|r| r.map_err(std::io::Error::other));
    builder.body(Body::from_stream(stream)).unwrap()
}
