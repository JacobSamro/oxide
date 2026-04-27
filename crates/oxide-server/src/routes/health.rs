use axum::http::{header, StatusCode};
use axum::response::IntoResponse;

use crate::metrics;

pub async fn ping() -> &'static str { "pong" }

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], r#"{"ok":true}"#)
}

pub async fn metrics() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        metrics::render(),
    )
}
