mod api;
mod auth;
mod config;
mod db;
mod error;
mod events;
mod models;
mod storage;

use std::sync::Arc;
use std::collections::HashSet;
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

    sync_storage_with_db(&storage, &pool).await?;

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

async fn sync_storage_with_db(
    storage: &storage::S3Storage,
    pool: &db::DbPool,
) -> anyhow::Result<()> {
    let db_keys = db::files::list_all_s3_keys(pool)
        .map_err(|e| anyhow::anyhow!("Failed to list DB keys: {e}"))?;
    let db_set: HashSet<&str> = db_keys.iter().map(|k| k.as_str()).collect();

    let s3_keys = storage
        .list_objects()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list S3 objects: {e}"))?;

    for s3_key in &s3_keys {
        if !db_set.contains(s3_key.as_str()) {
            tracing::warn!("Orphan S3 object, removing: {s3_key}");
            storage
                .delete(s3_key)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to delete orphan S3 object {s3_key}: {e}"))?;
        }
    }

    let orphan_count = s3_keys.len() - db_set.len();
    if orphan_count > 0 {
        tracing::info!("Sync complete: removed {orphan_count} orphan object(s) from S3");
    } else {
        tracing::info!("Sync complete: no orphan objects found");
    }

    Ok(())
}