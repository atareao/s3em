pub mod events;
pub mod files;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use anyhow::Result;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn create_pool(database_url: &str) -> Result<DbPool> {
    let manager = SqliteConnectionManager::file(database_url);
    let pool = Pool::builder().max_size(8).build(manager)?;

    let conn = pool.get()?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -64000;",
    )?;

    Ok(pool)
}

pub fn run_migrations(pool: &DbPool) -> Result<()> {
    let conn = pool.get()?;
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS files (
            id              TEXT PRIMARY KEY,
            name            TEXT NOT NULL,
            path            TEXT NOT NULL,
            size            INTEGER NOT NULL,
            content_type    TEXT NOT NULL DEFAULT 'application/octet-stream',
            etag            TEXT,
            checksum_sha256 TEXT,
            mode            INTEGER,
            uid             INTEGER,
            gid             INTEGER,
            username        TEXT,
            groupname       TEXT,
            mtime           TEXT,
            ctime           TEXT,
            atime           TEXT,
            device          INTEGER,
            inode           INTEGER,
            nlink           INTEGER,
            created_at      TEXT NOT NULL,
            updated_at      TEXT NOT NULL,
            deleted_at      TEXT
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_files_path_name ON files(path, name);

        CREATE TABLE IF NOT EXISTS events (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type      TEXT NOT NULL,
            resource_type   TEXT NOT NULL,
            resource_id     TEXT,
            payload         TEXT NOT NULL,
            created_at      TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
        CREATE INDEX IF NOT EXISTS idx_events_created ON events(created_at);
        ",
    )?;
    Ok(())
}