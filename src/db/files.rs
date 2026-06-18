use rusqlite::params;
use crate::db::DbPool;
use crate::models::{FileRecord, FileFilter};
use crate::error::AppError;

pub fn insert_file(pool: &DbPool, record: &FileRecord) -> Result<(), AppError> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO files (id, name, path, size, content_type, etag, checksum_sha256,
         mode, uid, gid, username, groupname, mtime, ctime, atime,
         device, inode, nlink, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        params![
            record.id, record.name, record.path, record.size,
            record.content_type, record.etag, record.checksum_sha256,
            record.mode, record.uid, record.gid,
            record.username, record.groupname,
            record.mtime, record.ctime, record.atime,
            record.device, record.inode, record.nlink,
            record.created_at, record.updated_at,
        ],
    )?;
    Ok(())
}

pub fn update_file(pool: &DbPool, id: &str, record: &FileRecord) -> Result<(), AppError> {
    let conn = pool.get()?;
    conn.execute(
        "UPDATE files SET name = ?1, path = ?2, size = ?3, content_type = ?4,
         etag = ?5, checksum_sha256 = ?6,
         mode = ?7, uid = ?8, gid = ?9, username = ?10, groupname = ?11,
         mtime = ?12, ctime = ?13, atime = ?14, device = ?15, inode = ?16, nlink = ?17,
         updated_at = ?18, deleted_at = NULL
         WHERE id = ?19",
        params![
            record.name, record.path, record.size, record.content_type,
            record.etag, record.checksum_sha256,
            record.mode, record.uid, record.gid,
            record.username, record.groupname,
            record.mtime, record.ctime, record.atime,
            record.device, record.inode, record.nlink,
            record.updated_at, id,
        ],
    )?;
    Ok(())
}

type ExistingFile = (String, Option<String>, Option<String>);

pub fn find_by_path_and_name(
    pool: &DbPool,
    path: &str,
    name: &str,
) -> Result<Option<ExistingFile>, AppError> {
    let conn = pool.get()?;
    let result = conn.query_row(
        "SELECT id, checksum_sha256, deleted_at FROM files WHERE path = ?1 AND name = ?2",
        params![path, name],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        )),
    );
    match result {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

pub fn get_by_id(pool: &DbPool, id: &str) -> Result<FileRecord, AppError> {
    let conn = pool.get()?;
    let record = conn.query_row(
        "SELECT id, name, path, size, content_type, etag, checksum_sha256,
         mode, uid, gid, username, groupname, mtime, ctime, atime,
         device, inode, nlink, created_at, updated_at, deleted_at
         FROM files WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
        row_to_file_record,
    ).map_err(|_| AppError::NotFound(format!("File {id} not found")))?;
    Ok(record)
}

pub fn list_files(pool: &DbPool, filters: &FileFilter) -> Result<Vec<FileRecord>, AppError> {
    let conn = pool.get()?;
    let limit = filters.limit.unwrap_or(50).min(200);
    let offset = filters.offset.unwrap_or(0);

    let mut sql = String::from(
        "SELECT id, name, path, size, content_type, etag, checksum_sha256,
         mode, uid, gid, username, groupname, mtime, ctime, atime,
         device, inode, nlink, created_at, updated_at, deleted_at
         FROM files WHERE deleted_at IS NULL",
    );
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref path_filter) = filters.path {
        param_values.push(Box::new(format!("{}%", path_filter)));
        sql.push_str(&format!(" AND path LIKE ?{}", param_values.len()));
    }
    if let Some(ref search) = filters.search {
        param_values.push(Box::new(format!("%{}%", search)));
        sql.push_str(&format!(" AND name LIKE ?{}", param_values.len()));
    }

    sql.push_str(" ORDER BY created_at DESC");
    sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    let mut stmt = conn.prepare(&sql)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(params_refs.as_slice(), row_to_file_record)?;
    let files: Vec<FileRecord> = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(files)
}

pub fn soft_delete(pool: &DbPool, id: &str, deleted_at: &str) -> Result<(), AppError> {
    let conn = pool.get()?;
    let updated = conn.execute(
        "UPDATE files SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
        params![deleted_at, id],
    )?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("File {id} not found")));
    }
    Ok(())
}

pub fn count_files(pool: &DbPool, filters: &FileFilter) -> Result<i64, AppError> {
    let conn = pool.get()?;
    let mut sql = String::from("SELECT COUNT(*) FROM files WHERE deleted_at IS NULL");
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref path_filter) = filters.path {
        param_values.push(Box::new(format!("{}%", path_filter)));
        sql.push_str(&format!(" AND path LIKE ?{}", param_values.len()));
    }
    if let Some(ref search) = filters.search {
        param_values.push(Box::new(format!("%{}%", search)));
        sql.push_str(&format!(" AND name LIKE ?{}", param_values.len()));
    }

    let mut stmt = conn.prepare(&sql)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();
    let count: i64 = stmt.query_row(params_refs.as_slice(), |row| row.get(0))?;
    Ok(count)
}

pub fn list_all_s3_keys(pool: &DbPool) -> Result<Vec<String>, AppError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT path, name FROM files")?;
    let rows = stmt.query_map([], |row| {
        let path: String = row.get(0)?;
        let name: String = row.get(1)?;
        Ok((path, name))
    })?;

    let keys: Vec<String> = rows
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|(path, name)| {
            let base = path.trim_end_matches('/');
            if base.is_empty() {
                name
            } else {
                format!("{}/{}", base, name)
            }
        })
        .collect();

    Ok(keys)
}

fn row_to_file_record(row: &rusqlite::Row) -> rusqlite::Result<FileRecord> {
    Ok(FileRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        size: row.get(3)?,
        content_type: row.get(4)?,
        etag: row.get(5)?,
        checksum_sha256: row.get(6)?,
        mode: row.get(7)?,
        uid: row.get(8)?,
        gid: row.get(9)?,
        username: row.get(10)?,
        groupname: row.get(11)?,
        mtime: row.get(12)?,
        ctime: row.get(13)?,
        atime: row.get(14)?,
        device: row.get(15)?,
        inode: row.get(16)?,
        nlink: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
        deleted_at: row.get(20)?,
    })
}