use anyhow::Result;
use serde_json;
use std::sync::Arc;
use tracing::{info, warn};

use crate::connection::pool::ConnectionPool;

pub struct NotificationManager {
    pool: Arc<ConnectionPool>,
}

impl NotificationManager {
    pub fn new(pool: Arc<ConnectionPool>) -> Self {
        Self { pool }
    }

    pub async fn send_notification(&self, title: &str, body: &str, device_id: &str) -> Result<()> {
        let msg = serde_json::json!({
            "type": "notification",
            "id": uuid::Uuid::new_v4().to_string(),
            "timestamp": chrono::Utc::now().timestamp(),
            "payload": {
                "title": title,
                "body": body,
            }
        });
        let bytes = serde_json::to_vec(&msg)?;

        if device_id.is_empty() {
            self.pool.broadcast(&bytes, None).await;
        } else if let Some(peer) = self.pool.get_peer(device_id).await {
            peer.send_encrypted(&bytes).await?;
        } else {
            anyhow::bail!("Device {} not connected", device_id);
        }
        Ok(())
    }

    /// Listen for Freedesktop notifications via D-Bus and forward to peers.
    pub async fn start_listening(&self) -> Result<tokio::task::JoinHandle<()>> {
        let pool = self.pool.clone();

        let handle = tokio::spawn(async move {
            // Connect to session bus for Freedesktop notifications
            let _connection = match zbus::Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to connect to D-Bus session: {}. Notification forwarding disabled.", e);
                    return;
                }
            };

            info!("D-Bus notification listener started");

            // ponytail: Real D-Bus notification monitoring via zbus signal streams
            // is non-trivial (requires implementing org.freedesktop.Notifications interface).
            // For now, notifications are sent via IPC (handoff notify).
            // Upgrade: implement NotificationProxy with zbus proxy macro.
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        });

        Ok(handle)
    }
}
