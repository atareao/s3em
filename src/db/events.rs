use rusqlite::params;
use crate::db::DbPool;
use crate::models::{AppEvent, EventRecord, EventHistoryFilter};
use crate::error::AppError;

pub struct EventRepository {
    pool: DbPool,
}

impl EventRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn persist(&self, event: &AppEvent) -> Result<(), AppError> {
        let conn = self.pool.get()?;
        let (event_type, resource_type, resource_id, payload) = match event {
            AppEvent::FileCreated { resource_id, payload } => {
                ("created", "file", resource_id.clone(), payload.clone())
            }
            AppEvent::FileUpdated { resource_id, payload } => {
                ("updated", "file", resource_id.clone(), payload.clone())
            }
            AppEvent::FileDeleted { resource_id, payload } => {
                ("deleted", "file", resource_id.clone(), payload.clone())
            }
        };

        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO events (event_type, resource_type, resource_id, payload, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_type, resource_type, resource_id, payload.to_string(), now],
        )?;
        Ok(())
    }

    pub fn list_events(
        &self,
        pool: &DbPool,
        filters: &EventHistoryFilter,
    ) -> Result<Vec<EventRecord>, AppError> {
        let conn = pool.get()?;
        let limit = filters.limit.unwrap_or(100).min(1000);
        let offset = filters.offset.unwrap_or(0);

        let mut sql = String::from(
            "SELECT id, event_type, resource_type, resource_id, payload, created_at FROM events WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref event_type) = filters.event_type {
            param_values.push(Box::new(event_type.clone()));
            sql.push_str(&format!(" AND event_type = ?{}", param_values.len()));
        }
        if let Some(ref since) = filters.since {
            param_values.push(Box::new(since.clone()));
            sql.push_str(&format!(" AND created_at >= ?{}", param_values.len()));
        }

        sql.push_str(" ORDER BY id DESC");
        sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(EventRecord {
                id: row.get(0)?,
                event_type: row.get(1)?,
                resource_type: row.get(2)?,
                resource_id: row.get(3)?,
                payload: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                created_at: row.get(5)?,
            })
        })?;
        let events: Vec<EventRecord> = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(events)
    }
}