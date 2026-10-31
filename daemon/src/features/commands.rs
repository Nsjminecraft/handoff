use anyhow::Result;
use serde_json;
use std::sync::Arc;
use tracing::{info, warn};

use crate::connection::pool::ConnectionPool;

pub struct CommandExecutor {
    pool: Arc<ConnectionPool>,
}

impl CommandExecutor {
    pub fn new(pool: Arc<ConnectionPool>) -> Self {
        Self { pool }
    }

    /// Execute a command locally (with whitelist).
    pub async fn execute_local(command: &str) -> Result<String> {
        // Whitelist of allowed commands for safety
        let allowed = ["ls", "pwd", "date", "uptime", "whoami", "hostname", "echo", "cat", "head", "tail", "wc"];
        let cmd_name = command.split_whitespace().next().unwrap_or("");
        if !allowed.contains(&cmd_name) {
            anyhow::bail!("Command not allowed: {}. Allowed: {:?}", cmd_name, allowed);
        }

        info!("Executing local command: {}", command);
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            anyhow::bail!("Command failed: {}", stderr);
        }
        Ok(stdout)
    }

    /// Send a command to a remote device for execution.
    pub async fn send_command(&self, command: &str, device_id: &str) -> Result<()> {
        let msg = serde_json::json!({
            "type": "command.request",
            "id": uuid::Uuid::new_v4().to_string(),
            "timestamp": chrono::Utc::now().timestamp(),
            "payload": {
                "command": command,
            }
        });
        let bytes = serde_json::to_vec(&msg)?;

        if let Some(peer) = self.pool.get_peer(device_id).await {
            peer.send_encrypted(&bytes).await?;
            info!("Sent command to {}: {}", device_id, command);
        } else {
            anyhow::bail!("Device {} not connected", device_id);
        }
        Ok(())
    }
}
