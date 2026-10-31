pub mod clipboard_db;
pub mod devices;

use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

pub struct Storage {
    conn: Connection,
    data_dir: PathBuf,
}

impl Storage {
    pub fn new(data_dir: &PathBuf) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("handoff.db");
        let conn = Connection::open(db_path)?;

        // Initialize tables
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                addr TEXT NOT NULL DEFAULT '',
                public_key BLOB NOT NULL,
                paired_at INTEGER NOT NULL,
                last_seen INTEGER
            );

            CREATE TABLE IF NOT EXISTS clipboard_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                source_device TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (source_device) REFERENCES devices(id)
            );
            ",
        )?;

        Ok(Self {
            conn,
            data_dir: data_dir.clone(),
        })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn list_devices(&self) -> Vec<devices::Device> {
        devices::DeviceStore::list(&self.conn).unwrap_or_default()
    }

    pub fn remove_device(&self, id: &str) -> Result<()> {
        devices::DeviceStore::delete(&self.conn, id)
    }

    pub fn save_device(&self, device: &devices::Device) -> Result<()> {
        devices::DeviceStore::save(&self.conn, device)
    }
}
