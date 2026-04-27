// npm-compatible auth endpoints. The npm CLI does:
//   PUT  /-/user/org.couchdb.user::<name>   { name, password, email }   → returns { token }
//   GET  /-/whoami                                                      → returns { username }
//   DELETE /-/user/token/:token                                         → revoke
//
// (CouchDB-style URLs are historical; npm's public registry still uses them.)

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::auth::{extract_bearer, issue_token, revoke_token, user_from_token, verify_password};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginBody {
    pub name: String,
    pub password: String,
    #[serde(default)]
    pub email: Option<String>,
}

pub async fn login_or_create_user(
    State(state): State<Arc<AppState>>,
    Path(_userpath): Path<String>,
    Json(body): Json<LoginBody>,
) -> Response {
    // The path looks like `org.couchdb.user:alice` or just `alice`; we don't enforce a
    // match — the body's `name` is what npm uses to authenticate.

    let db = match state.db.as_ref() {
        Some(d) => d.clone(),
        None => return server_error("publish disabled (no writable db)"),
    };

    let user = match verify_password(&db, &body.name, &body.password) {
        Ok(u) => u,
        Err(_) => return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "Incorrect or missing password."})),
        ).into_response(),
    };

    let token = match issue_token(&db, user.id, Some("npm-cli")) {
        Ok(t) => t,
        Err(e) => return server_error(&e.to_string()),
    };

    (
        StatusCode::CREATED,
        Json(json!({
            "ok": true,
            "id": format!("org.couchdb.user:{}", user.name),
            "rev": "1-0",
            "token": token,
        })),
    ).into_response()
}

pub async fn whoami(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let Some(token) = extract_bearer(&headers) else {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "missing token"}))).into_response();
    };
    let Some(db) = state.db.as_ref() else {
        return server_error("publish disabled");
    };
    match user_from_token(db, &token) {
        Ok(u) => Json(json!({"username": u.name})).into_response(),
        Err(_) => (StatusCode::UNAUTHORIZED, Json(json!({"error": "invalid token"}))).into_response(),
    }
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Response {
    if let Some(db) = state.db.as_ref() {
        let _ = revoke_token(db, &token);
    }
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], r#"{"ok":true}"#).into_response()
}

fn server_error(msg: &str) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": msg}))).into_response()
}
