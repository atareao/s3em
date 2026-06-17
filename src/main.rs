mod api;
mod auth;
mod config;
mod db;
mod error;
mod events;
mod models;
mod storage;

use std::sync::Arc;
use axum::serve;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = config::Config::from_env();
    tracing::info!("Starting server on {}", cfg.server_addr());

    let pool = db::create_pool(&cfg.database_url)?;
    db::run_migrations(&pool)?;
    tracing::info!("Database ready: {}", cfg.database_url);

    let storage = storage::S3Storage::new(&cfg).await;
    storage.ensure_bucket_exists().await.map_err(|e| anyhow::anyhow!(e))?;
    tracing::info!("S3 bucket ready: {}", cfg.s3_bucket);

    let event_repo = Arc::new(db::events::EventRepository::new(pool.clone()));
    let event_bus = events::EventBus::new(1024, event_repo);

    let state = Arc::new(api::AppState {
        pool,
        storage,
        event_bus,
        jwt_secret: cfg.jwt_secret.clone(),
        master_api_key: cfg.master_api_key.clone(),
    });
    let app = api::build_router(state);

    let listener = TcpListener::bind(cfg.server_addr()).await?;
    tracing::info!("Listening on {}", cfg.server_addr());
    serve(listener, app).await?;

    Ok(())
}