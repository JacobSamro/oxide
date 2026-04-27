use std::time::Instant;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

use crate::config::AuditMode;
use crate::metrics::METRICS;
use crate::state::AppState;

pub async fn audit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let started = Instant::now();
    let result = match state.cfg.audit.mode {
        AuditMode::Disabled => {
            METRICS.audit.with_label_values(&["disabled"]).inc();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                r#"{"actions":[],"advisories":{},"muted":[],"metadata":{"vulnerabilities":{"info":0,"low":0,"moderate":0,"high":0,"critical":0},"dependencies":0,"devDependencies":0,"optionalDependencies":0,"totalDependencies":0}}"#,
            ).into_response()
        }
        AuditMode::Empty => {
            METRICS.audit.with_label_values(&["empty"]).inc();
            (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], "{}").into_response()
        }
        AuditMode::Proxy => {
            // Best-effort proxy with isolated short timeout; fall back to empty if it fails.
            let url = format!("{}/-/npm/v1/security/audits", state.metadata.upstream.url);
            let req = state.metadata.upstream.client
                .post(&url)
                .timeout(std::time::Duration::from_secs(10))
                .header(header::CONTENT_TYPE, headers.get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok()).unwrap_or("application/json"))
                .body(body);
            match req.send().await {
                Ok(res) => {
                    let status = res.status();
                    let ct = res.headers().get(header::CONTENT_TYPE).cloned();
                    let bytes = res.bytes().await.unwrap_or_default();
                    METRICS.audit.with_label_values(&[status.as_str()]).inc();
                    let mut resp = Response::builder().status(status);
                    if let Some(ct) = ct { resp = resp.header(header::CONTENT_TYPE, ct); }
                    resp.body(Body::from(bytes)).unwrap_or_else(|_| empty_audit())
                }
                Err(_) => {
                    METRICS.audit.with_label_values(&["upstream_error"]).inc();
                    empty_audit()
                }
            }
        }
    };

    METRICS.audit_latency.with_label_values(&["any"]).observe(started.elapsed().as_secs_f64());
    result
}

fn empty_audit() -> Response {
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], "{}").into_response()
}
