use std::sync::Arc;

use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::state::AppState;

mod audit;
mod health;
mod npm_auth;
mod npm_publish;
mod registry;

pub fn router(state: Arc<AppState>) -> Router {
    // Higher request body cap for publishes — npm payloads include a base64 tarball.
    let publish_limit = axum::extract::DefaultBodyLimit::max(256 * 1024 * 1024);

    Router::new()
        .route("/-/ping", get(health::ping))
        .route("/-/health", get(health::health))
        .route("/metrics", get(health::metrics))

        // npm audit
        .route("/-/npm/v1/security/audits", post(audit::audit))
        .route("/-/npm/v1/security/audits/quick", post(audit::audit))
        .route("/-/npm/v1/security/advisories/bulk", post(audit::audit))

        // npm auth. The login URL is /-/user/org.couchdb.user:<name>, which lands here as a
        // single path segment captured by `:userpath`. The handler ignores the segment value
        // and authenticates from the body (npm sends name+password there anyway).
        .route("/-/user/:userpath", put(npm_auth::login_or_create_user))
        .route("/-/whoami", get(npm_auth::whoami))
        .route("/-/user/token/:token", delete(npm_auth::logout))

        // Tarballs (must come before package metadata catch-all).
        .route("/:package/-/:file", get(registry::tarball_unscoped))
        .route("/:scope/:package/-/:file", get(registry::tarball_scoped))

        // Cache management.
        .route("/-/oxide/cache/:package", delete(registry::invalidate))
        .route("/-/oxide/reload", post(registry::reload))

        // Package metadata + publish on the same paths (different methods).
        .route("/:package",
               get(registry::metadata_unscoped).put(npm_publish::publish_unscoped))
        .route("/:scope/:package",
               get(registry::metadata_scoped).put(npm_publish::publish_scoped))

        .layer(publish_limit)
        .with_state(state)
}
