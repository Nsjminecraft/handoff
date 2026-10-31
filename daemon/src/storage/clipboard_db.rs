use anyhow::Result;
use rusqlite::Connection;

use crate::storage::devices::Device;

pub struct ClipboardDb;

impl ClipboardDb {
    pub fn save_clipboard(
        conn: &Connection,
        content: &str,
        mime_type: &str,
        source_device: Option<&str>,
    ) -> Result<i64> {
        conn.execute(
            "INSERT INTO clipboard_history (content, mime_type, source_device, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![content, mime_type, source_device, chrono::Utc::now().timestamp()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_history(conn: &Connection, limit: usize) -> Result<Vec<ClipboardEntry>> {
        let mut stmt = conn.prepare(
            "SELECT id, content, mime_type, source_device, created_at FROM clipboard_history ORDER BY created_at DESC LIMIT ?1",
        )?;
        let entries = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(ClipboardEntry {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    mime_type: row.get(2)?,
                    source_device: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }
}

#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content: String,
    pub mime_type: String,
    pub source_device: Option<String>,
    pub created_at: i64,
}
