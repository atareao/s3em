use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: i64,
    pub content_type: String,
    pub etag: Option<String>,
    pub checksum_sha256: Option<String>,
    pub mode: Option<i64>,
    pub uid: Option<i64>,
    pub gid: Option<i64>,
    pub username: Option<String>,
    pub groupname: Option<String>,
    pub mtime: Option<String>,
    pub ctime: Option<String>,
    pub atime: Option<String>,
    pub device: Option<i64>,
    pub inode: Option<i64>,
    pub nlink: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMetadata {
    pub path: String,
    pub mode: Option<i64>,
    pub uid: Option<i64>,
    pub gid: Option<i64>,
    pub username: Option<String>,
    pub groupname: Option<String>,
    pub mtime: Option<String>,
    pub ctime: Option<String>,
    pub atime: Option<String>,
    pub device: Option<i64>,
    pub inode: Option<i64>,
    pub nlink: Option<i64>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {
    FileCreated {
        resource_id: String,
        payload: serde_json::Value,
    },
    FileUpdated {
        resource_id: String,
        payload: serde_json::Value,
    },
    FileDeleted {
        resource_id: String,
        payload: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: i64,
    pub event_type: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct FileFilter {
    pub path: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct EventHistoryFilter {
    pub event_type: Option<String>,
    pub since: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}