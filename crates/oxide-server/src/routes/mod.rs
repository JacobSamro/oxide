use std::sync::Arc;

use axum::routing::{get, post, delete};
use axum::Router;

use crate::state::AppState;

mod registry;
mod audit;
mod health;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/-/ping", get(health::ping))
        .route("/-/health", get(health::health))
        .route("/metrics", get(health::metrics))
        // npm audit endpoints — multiple paths used by npm/pnpm/yarn.
        .route("/-/npm/v1/security/audits", post(audit::audit))
        .route("/-/npm/v1/security/audits/quick", post(audit::audit))
        .route("/-/npm/v1/security/advisories/bulk", post(audit::audit))
        // Tarballs (must come before package metadata catch-all).
        .route("/:package/-/:file", get(registry::tarball_unscoped))
        .route("/:scope/:package/-/:file", get(registry::tarball_scoped))
        // Cache management.
        .route("/-/oxide/cache/:package", delete(registry::invalidate))
        .route("/-/oxide/reload", post(registry::reload))
        // Package metadata.
        .route("/:package", get(registry::metadata_unscoped))
        .route("/:scope/:package", get(registry::metadata_scoped))
        .with_state(state)
}
