use anyhow::Result;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::connection::pool::ConnectionPool;
use crate::storage::Storage;

pub struct ClipboardManager {
    pool: Arc<ConnectionPool>,
    storage: Arc<std::sync::Mutex<Storage>>,
}

impl ClipboardManager {
    pub fn new(pool: Arc<ConnectionPool>, storage: Arc<std::sync::Mutex<Storage>>) -> Self {
        Self { pool, storage }
    }

    pub async fn start(&self) -> Result<JoinHandle<()>> {
        info!("Clipboard manager starting");

        let pool = self.pool.clone();
        let storage = self.storage.clone();

        let handle = tokio::spawn(async move {
            let mut last_content = String::new();

            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Read current clipboard
                let content = match cli_clipboard::get_contents() {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                if content == last_content || content.is_empty() {
                    continue;
                }

                debug!("Clipboard changed: {} chars", content.len());
                last_content = content.clone();

                // Save to history
                {
                    let storage = storage.lock().unwrap();
                    if let Err(e) = storage.connection().execute(
                        "INSERT INTO clipboard_history (content, mime_type, created_at) VALUES (?1, 'text/plain', ?2)",
                        rusqlite::params![content, chrono::Utc::now().timestamp()],
                    ) {
                        warn!("Failed to save clipboard: {}", e);
                    }
                }

                // Broadcast to peers
                let msg = serde_json::json!({
                    "type": "clipboard.sync",
                    "id": uuid::Uuid::new_v4().to_string(),
                    "timestamp": chrono::Utc::now().timestamp(),
                    "payload": {
                        "mime": "text/plain",
                        "data": content,
                    }
                });
                if let Ok(json) = serde_json::to_string(&msg) {
                    pool.broadcast(json.as_bytes(), None).await;
                }
            }
        });

        Ok(handle)
    }
}
