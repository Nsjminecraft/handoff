use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tracing::{error, info};

use crate::config::Config;
use crate::connection::pool::ConnectionPool;
use crate::storage::Storage;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcRequest {
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "devices")]
    Devices,
    #[serde(rename = "clipboard.get")]
    ClipboardGet,
    #[serde(rename = "clipboard.set")]
    ClipboardSet { text: String },
    #[serde(rename = "notify")]
    Notify { title: String, body: String },
    #[serde(rename = "handoff.url")]
    HandoffUrl { url: String, device_id: String },
    #[serde(rename = "command.exec")]
    CommandExec { command: String, device_id: String },
    #[serde(rename = "pair")]
    Pair,
    #[serde(rename = "unpair")]
    Unpair { device_id: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum IpcResponse {
    #[serde(rename = "status")]
    Status { peer_count: usize, running: bool },
    #[serde(rename = "devices")]
    Devices { devices: Vec<DeviceInfo> },
    #[serde(rename = "clipboard")]
    Clipboard { text: Option<String> },
    #[serde(rename = "pairing")]
    Pairing { qr_code: String, pairing_data: String },
    #[serde(rename = "ok")]
    Ok { message: String },
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub addr: String,
    pub connected: bool,
}

pub struct IpcServer {
    socket_path: PathBuf,
    pool: Arc<ConnectionPool>,
    storage: Arc<std::sync::Mutex<Storage>>,
    config: Config,
}

impl IpcServer {
    pub fn new(
        socket_path: PathBuf,
        pool: Arc<ConnectionPool>,
        storage: Arc<std::sync::Mutex<Storage>>,
        config: Config,
    ) -> Self {
        Self { socket_path, pool, storage, config }
    }

    pub async fn start(&self) -> Result<tokio::task::JoinHandle<()>> {
        // Remove existing socket file
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;
        info!("IPC server listening on {:?}", self.socket_path);

        let pool = self.pool.clone();
        let storage = self.storage.clone();
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        let pool = pool.clone();
                        let storage = storage.clone();
                        let config = config.clone();
                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_client(stream, pool, storage, config).await {
                                error!("IPC client error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("IPC accept error: {}", e);
                    }
                }
            }
        });

        Ok(handle)
    }

    async fn handle_client(
        stream: tokio::net::UnixStream,
        pool: Arc<ConnectionPool>,
        storage: Arc<std::sync::Mutex<Storage>>,
        config: Config,
    ) -> Result<()> {
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        while reader.read_line(&mut line).await? > 0 {
            let response = match serde_json::from_str::<IpcRequest>(&line) {
                Ok(request) => Self::handle_request(request, &pool, &storage, &config).await,
                Err(e) => IpcResponse::Error {
                    message: format!("Invalid request: {}", e),
                },
            };

            let mut response_json = serde_json::to_string(&response)?;
            response_json.push('\n');
            writer.write_all(response_json.as_bytes()).await?;
            writer.flush().await?;

            line.clear();
        }

        Ok(())
    }

    async fn handle_request(
        request: IpcRequest,
        pool: &Arc<ConnectionPool>,
        storage: &Arc<std::sync::Mutex<Storage>>,
        config: &Config,
    ) -> IpcResponse {
        match request {
            IpcRequest::Status => {
                let peer_count = pool.peer_count().await;
                IpcResponse::Status {
                    peer_count,
                    running: true,
                }
            }
            IpcRequest::Devices => {
                let devices = {
                    let storage_guard = storage.lock().unwrap();
                    storage_guard.list_devices()
                };
                let peer_ids = pool.peer_ids().await;
                let devices: Vec<DeviceInfo> = devices
                    .into_iter()
                    .map(|d| DeviceInfo {
                        id: d.id.clone(),
                        name: d.name.clone(),
                        addr: d.addr.clone(),
                        connected: peer_ids.iter().any(|id| id.contains(&d.id)),
                    })
                    .collect();
                IpcResponse::Devices { devices }
            }
            IpcRequest::ClipboardGet => {
                let text = cli_clipboard::get_contents().ok();
                IpcResponse::Clipboard { text }
            }
            IpcRequest::ClipboardSet { text } => {
                let set_result: Result<(), String> = cli_clipboard::set_contents(text.clone())
                    .map_err(|e| e.to_string());
                match set_result {
                    Ok(_) => {
                        // Broadcast to peers
                        let msg = serde_json::json!({
                            "type": "clipboard.sync",
                            "id": uuid::Uuid::new_v4().to_string(),
                            "timestamp": chrono::Utc::now().timestamp(),
                            "payload": {
                                "mime": "text/plain",
                                "data": text,
                            }
                        });
                        let bytes = serde_json::to_vec(&msg).unwrap();
                        pool.broadcast(&bytes, None).await;
                        IpcResponse::Ok { message: "Clipboard set and broadcast".to_string() }
                    }
                    Err(e) => IpcResponse::Error {
                        message: format!("Failed to set clipboard: {}", e),
                    },
                }
            }
            IpcRequest::Notify { title, body } => {
                // Send notification to all peers
                let msg = serde_json::json!({
                    "type": "notification",
                    "id": uuid::Uuid::new_v4().to_string(),
                    "timestamp": chrono::Utc::now().timestamp(),
                    "payload": {
                        "title": title,
                        "body": body,
                    }
                });
                let bytes = serde_json::to_vec(&msg).unwrap();
                pool.broadcast(&bytes, None).await;
                IpcResponse::Ok { message: "Notification sent".to_string() }
            }
            IpcRequest::Pair => {
                // Generate pairing data with device info and Noise public key
                let noise_pubkey = pool.noise_context().public_key();
                let pairing_data = serde_json::json!({
                    "device_id": config.device_id,
                    "device_name": config.device_name,
                    "ws_port": config.ws_port,
                    "noise_pubkey": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, noise_pubkey),
                });
                let pairing_json = serde_json::to_string(&pairing_data).unwrap();

                // Generate QR code as ASCII art
                let qr = match qrcode::QrCode::new(pairing_json.as_bytes()) {
                    Ok(qr) => qr,
                    Err(e) => {
                        return IpcResponse::Error {
                            message: format!("Failed to generate QR code: {}", e),
                        };
                    }
                };
                let qr_ascii = qr.render::<char>()
                    .light_color(' ')
                    .dark_color('#')
                    .build();

                IpcResponse::Pairing {
                    qr_code: qr_ascii,
                    pairing_data: pairing_json,
                }
            }
            IpcRequest::Unpair { device_id } => {
                let result = {
                    let storage_guard = storage.lock().unwrap();
                    storage_guard.remove_device(&device_id)
                };
                match result {
                    Ok(_) => IpcResponse::Ok { message: format!("Device {} unpaired", device_id) },
                    Err(e) => IpcResponse::Error {
                        message: format!("Failed to unpair: {}", e),
                    },
                }
            }
            IpcRequest::HandoffUrl { url, device_id } => {
                let msg = serde_json::json!({
                    "type": "handoff.url",
                    "id": uuid::Uuid::new_v4().to_string(),
                    "timestamp": chrono::Utc::now().timestamp(),
                    "payload": { "url": url }
                });
                let bytes = serde_json::to_vec(&msg).unwrap();
                if let Some(peer) = pool.get_peer(&device_id).await {
                    match peer.send_encrypted(&bytes).await {
                        Ok(_) => IpcResponse::Ok { message: format!("URL sent to {}", device_id) },
                        Err(e) => IpcResponse::Error { message: format!("Failed to send: {}", e) },
                    }
                } else {
                    IpcResponse::Error { message: format!("Device {} not connected", device_id) }
                }
            }
            IpcRequest::CommandExec { command, device_id } => {
                let msg = serde_json::json!({
                    "type": "command.request",
                    "id": uuid::Uuid::new_v4().to_string(),
                    "timestamp": chrono::Utc::now().timestamp(),
                    "payload": { "command": command }
                });
                let bytes = serde_json::to_vec(&msg).unwrap();
                if let Some(peer) = pool.get_peer(&device_id).await {
                    match peer.send_encrypted(&bytes).await {
                        Ok(_) => IpcResponse::Ok { message: format!("Command sent to {}", device_id) },
                        Err(e) => IpcResponse::Error { message: format!("Failed to send: {}", e) },
                    }
                } else {
                    IpcResponse::Error { message: format!("Device {} not connected", device_id) }
                }
            }
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
