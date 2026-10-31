use anyhow::Result;
use serde_json;
use std::sync::Arc;
use tracing::{info, warn};

use crate::connection::pool::ConnectionPool;

pub struct HandoffManager {
    pool: Arc<ConnectionPool>,
}

impl HandoffManager {
    pub fn new(pool: Arc<ConnectionPool>) -> Self {
        Self { pool }
    }

    /// Send a URL to a remote device to open.
    pub async fn open_url(&self, url: &str, device_id: &str) -> Result<()> {
        let msg = serde_json::json!({
            "type": "handoff.url",
            "id": uuid::Uuid::new_v4().to_string(),
            "timestamp": chrono::Utc::now().timestamp(),
            "payload": {
                "url": url,
            }
        });
        let bytes = serde_json::to_vec(&msg)?;

        if let Some(peer) = self.pool.get_peer(device_id).await {
            peer.send_encrypted(&bytes).await?;
            info!("Sent URL {} to device {}", url, device_id);
        } else {
            anyhow::bail!("Device {} not connected", device_id);
        }
        Ok(())
    }

    /// Handle an incoming URL handoff — open locally with xdg-open.
    pub async fn handle_incoming_url(url: &str) -> Result<()> {
        info!("Opening handed-off URL: {}", url);
        let status = tokio::process::Command::new("xdg-open")
            .arg(url)
            .status()
            .await?;
        if !status.success() {
            warn!("xdg-open exited with status {}", status);
        }
        Ok(())
    }
}
