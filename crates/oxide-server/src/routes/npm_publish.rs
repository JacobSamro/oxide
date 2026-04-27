// PUT /:package and PUT /:scope/:package — accept npm publish payloads.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use tracing::{info, warn};

use crate::auth::{extract_bearer, user_from_token};
use crate::publish::handle_publish;
use crate::state::AppState;

pub async fn publish_unscoped(
    State(state): State<Arc<AppState>>,
    Path(package): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    publish(state, decode_pkg(&package), headers, body).await
}

pub async fn publish_scoped(
    State(state): State<Arc<AppState>>,
    Path((scope, name)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let scope = if scope.starts_with('@') { scope } else { format!("@{scope}") };
    let pkg = format!("{scope}/{name}");
    publish(state, pkg, headers, body).await
}

async fn publish(
    state: Arc<AppState>,
    package_name: String,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(db) = state.db.as_ref().cloned() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "publish disabled (no writable db)");
    };
    let Some(token) = extract_bearer(&headers) else {
        return err(StatusCode::UNAUTHORIZED, "missing bearer token");
    };
    let user = match user_from_token(&db, &token) {
        Ok(u) => u,
        Err(_) => return err(StatusCode::UNAUTHORIZED, "invalid token"),
    };

    let Some(local) = state.local.as_ref() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "local store unavailable");
    };

    match handle_publish(db, local, user.clone(), package_name.clone(), body).await {
        Ok(out) => {
            // Drop any cached upstream metadata for this name — the local copy now wins.
            state.metadata.invalidate(&out.package_name).await;
            info!(pkg=%out.package_name, ver=%out.version, by=%user.name, "publish ok");
            (StatusCode::CREATED, Json(json!({"ok": true, "id": out.package_name, "rev": "1-1"})))
                .into_response()
        }
        Err(e) => {
            warn!(?e, pkg=%package_name, "publish failed");
            err(StatusCode::BAD_REQUEST, &e.to_string())
        }
    }
}

fn decode_pkg(s: &str) -> String { s.replace("%2f", "/").replace("%2F", "/") }

fn err(code: StatusCode, msg: &str) -> Response {
    (code, Json(json!({"error": msg, "reason": msg}))).into_response()
}
