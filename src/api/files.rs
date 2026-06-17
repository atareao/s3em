use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, put},
    Json, Router,
};
use sha2::{Digest, Sha256};
use aws_sdk_s3::primitives::ByteStream;

use crate::db;
use crate::error::AppError;
use crate::api::SharedState;
use crate::models::{FileRecord, FileFilter, UploadMetadata};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/upload", put(upload_idempotent))
        .route("/", get(list_files))
        .route("/{id}", get(get_file))
        .route("/{id}/download", get(download_file))
        .route("/{id}", delete(delete_file))
}

fn s3_key(path: &str, name: &str) -> String {
    let base = path.trim_end_matches('/');
    if base.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", base, name)
    }
}

async fn upload_idempotent(
    State(state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let mut file_name = String::new();
    let mut file_data = Vec::new();
    let mut content_type = "application/octet-stream".to_string();
    let mut metadata: Option<UploadMetadata> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("Invalid multipart".into()))?
    {
        match field.name() {
            Some("file") => {
                file_name = field.file_name().unwrap_or("unknown").to_string();
                content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                file_data = field
                    .bytes()
                    .await
                    .map_err(|_| AppError::BadRequest("Failed to read file".into()))?
                    .to_vec();
            }
            Some("metadata") => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| AppError::BadRequest("Invalid metadata".into()))?;
                metadata = serde_json::from_str(&text).ok();
            }
            _ => {}
        }
    }

    if file_name.is_empty() || file_data.is_empty() {
        return Err(AppError::BadRequest(
            "file field and file name are required".into(),
        ));
    }

    let meta = metadata.unwrap_or(UploadMetadata {
        path: String::new(),
        mode: None,
        uid: None,
        gid: None,
        username: None,
        groupname: None,
        mtime: None,
        ctime: None,
        atime: None,
        device: None,
        inode: None,
        nlink: None,
    });

    let s3_key = s3_key(&meta.path, &file_name);
    let checksum = {
        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        hex::encode(hasher.finalize())
    };

    let now = chrono::Utc::now().to_rfc3339();

    let existing =
        db::files::find_by_path_and_name(&state.pool, &meta.path, &file_name)?;

    match existing {
        Some((_existing_id, Some(existing_checksum), _))
            if existing_checksum == checksum =>
        {
            Ok(StatusCode::NO_CONTENT.into_response())
        }
        Some((existing_id, _, _)) => {
            let body = ByteStream::from(file_data.clone());
            let etag = state.storage.upload(&s3_key, body, &content_type).await
                .map_err(AppError::Internal)?;

            let record = FileRecord {
                id: existing_id.clone(),
                name: file_name,
                path: meta.path,
                size: file_data.len() as i64,
                content_type,
                etag: Some(etag),
                checksum_sha256: Some(checksum),
                mode: meta.mode,
                uid: meta.uid,
                gid: meta.gid,
                username: meta.username,
                groupname: meta.groupname,
                mtime: meta.mtime,
                ctime: meta.ctime,
                atime: meta.atime,
                device: meta.device,
                inode: meta.inode,
                nlink: meta.nlink,
                created_at: now.clone(),
                updated_at: now,
                deleted_at: None,
            };

            db::files::update_file(&state.pool, &existing_id, &record)?;

            state
                .event_bus
                .emit(crate::models::AppEvent::FileUpdated {
                    resource_id: existing_id,
                    payload: serde_json::to_value(&record).unwrap(),
                })
                .await;

            Ok((StatusCode::OK, Json(record)).into_response())
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            let body = ByteStream::from(file_data.clone());
            let etag = state.storage.upload(&s3_key, body, &content_type).await
                .map_err(AppError::Internal)?;

            let record = FileRecord {
                id: id.clone(),
                name: file_name,
                path: meta.path,
                size: file_data.len() as i64,
                content_type,
                etag: Some(etag),
                checksum_sha256: Some(checksum),
                mode: meta.mode,
                uid: meta.uid,
                gid: meta.gid,
                username: meta.username,
                groupname: meta.groupname,
                mtime: meta.mtime,
                ctime: meta.ctime,
                atime: meta.atime,
                device: meta.device,
                inode: meta.inode,
                nlink: meta.nlink,
                created_at: now.clone(),
                updated_at: now,
                deleted_at: None,
            };

            db::files::insert_file(&state.pool, &record)?;

            state
                .event_bus
                .emit(crate::models::AppEvent::FileCreated {
                    resource_id: id,
                    payload: serde_json::to_value(&record).unwrap(),
                })
                .await;

            Ok((StatusCode::CREATED, Json(record)).into_response())
        }
    }
}

async fn list_files(
    State(state): State<SharedState>,
    Query(filters): Query<FileFilter>,
) -> Result<Json<Vec<FileRecord>>, AppError> {
    let files = db::files::list_files(&state.pool, &filters)?;
    Ok(Json(files))
}

async fn get_file(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<FileRecord>, AppError> {
    let record = db::files::get_by_id(&state.pool, &id)?;
    Ok(Json(record))
}

async fn download_file(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, [(String, String); 2], Vec<u8>), AppError> {
    let record = db::files::get_by_id(&state.pool, &id)?;
    let s3_key = s3_key(&record.path, &record.name);
    let output = state
        .storage
        .download(&s3_key)
        .await
        .map_err(AppError::Internal)?;

    let content_type = output
        .content_type
        .unwrap_or_else(|| "application/octet-stream".into());

    let data = output
        .body
        .collect()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((
        StatusCode::OK,
        [
            ("content-type".to_string(), content_type),
            (
                "content-disposition".to_string(),
                format!("attachment; filename=\"{}\"", record.name),
            ),
        ],
        data.to_vec(),
    ))
}

async fn delete_file(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let record = db::files::get_by_id(&state.pool, &id)?;
    let s3_key = s3_key(&record.path, &record.name);

    state
        .storage
        .delete(&s3_key)
        .await
        .map_err(AppError::Internal)?;

    let now = chrono::Utc::now().to_rfc3339();
    db::files::soft_delete(&state.pool, &id, &now)?;

    state
        .event_bus
        .emit(crate::models::AppEvent::FileDeleted {
            resource_id: id,
            payload: serde_json::json!({ "name": record.name, "path": record.path }),
        })
        .await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}