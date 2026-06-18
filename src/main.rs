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
use std::sync::atomic::{AtomicUsize, Ordering};
use axum::serve;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use futures_util::future::join_all;
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
    let addr = listener.local_addr()?;
    tracing::info!("Listening on {}", addr);
    tracing::info!("🚀 Server ready on port {}", addr.port());
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

    let orphans: Vec<&str> = s3_keys.iter().filter_map(|k| {
        if !db_set.contains(k.as_str()) {
            Some(k.as_str())
        } else {
            None
        }
    }).collect();

    if orphans.is_empty() {
        tracing::info!("Sync complete: no orphan objects found");
        return Ok(());
    }

    let semaphore = Arc::new(Semaphore::new(10));
    let deleted = Arc::new(AtomicUsize::new(0));
    let total = orphans.len();

    let tasks: Vec<_> = orphans.into_iter().map(|s3_key| {
        let sem = Arc::clone(&semaphore);
        let del = Arc::clone(&deleted);
        let key = s3_key.to_string();
        async move {
            let _permit = sem.acquire().await.unwrap();
            storage.delete(&key).await.map_err(|e| {
                anyhow::anyhow!("Failed to delete orphan S3 object {key}: {e}")
            })?;
            del.fetch_add(1, Ordering::Relaxed);
            tracing::debug!("Deleted orphan S3 object: {key}");
            Ok::<_, anyhow::Error>(())
        }
    }).collect();

    let results: Vec<anyhow::Result<()>> = join_all(tasks).await;

    let mut errors = 0;
    for r in &results {
        if let Err(e) = r {
            tracing::error!("{e}");
            errors += 1;
        }
    }

    let deleted_count = deleted.load(Ordering::Relaxed);
    tracing::info!(
        "Sync complete: removed {deleted_count}/{total} orphan object(s) from S3 ({errors} errors)",
    );

    if errors > 0 {
        Err(anyhow::anyhow!("{errors} orphan deletion(s) failed"))
    } else {
        Ok(())
    }
}