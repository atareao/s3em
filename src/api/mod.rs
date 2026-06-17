pub mod events;
pub mod files;
pub mod health;

use axum::{middleware, Router};
use std::sync::Arc;
use crate::db::DbPool;
use crate::events::EventBus;
use crate::storage::S3Storage;

pub struct AppState {
    pub pool: DbPool,
    pub storage: S3Storage,
    pub event_bus: EventBus,
    pub jwt_secret: String,
    pub master_api_key: String,
}

pub type SharedState = Arc<AppState>;

pub fn build_router(state: SharedState) -> Router {
    let protected_routes = Router::new()
        .nest("/api/files", crate::api::files::routes())
        .nest("/api/events", crate::api::events::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::auth_middleware,
        ));

    Router::new()
        .route("/api/health", axum::routing::get(crate::api::health::health))
        .nest("/api/auth", crate::auth::routes())
        .merge(protected_routes)
        .with_state(state)
}