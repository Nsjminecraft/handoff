use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{StreamExt, SinkExt};
use tracing::{debug, error, info, warn};

use super::client::PeerConnection;
use super::pool::ConnectionPool;
use crate::storage::Storage;
use crate::storage::devices::{Device, DeviceStore};

pub struct WebSocketServer {
    port: u16,
    pool: Arc<ConnectionPool>,
    storage: Arc<std::sync::Mutex<Storage>>,
}

impl WebSocketServer {
    pub fn new(port: u16, pool: Arc<ConnectionPool>, storage: Arc<std::sync::Mutex<Storage>>) -> Self {
        Self { port, pool, storage }
    }

    pub async fn start(&self) -> anyhow::Result<JoinHandle<()>> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        info!("WebSocket server listening on {}", addr);

        let pool = self.pool.clone();
        let storage = self.storage.clone();

        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        debug!("New connection from {}", addr);
                        let pool = pool.clone();
                        let storage = storage.clone();

                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_connection(stream, addr.to_string(), pool, storage).await {
                                error!("Connection error from {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                    }
                }
            }
        });

        Ok(handle)
    }

    async fn handle_connection(
        stream: tokio::net::TcpStream,
        remote_addr: String,
        pool: Arc<ConnectionPool>,
        storage: Arc<std::sync::Mutex<Storage>>,
    ) -> Result<(), anyhow::Error> {
        let ws_stream = accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();

        // Look up any known device at this address for trust binding
        let known_pubkey = {
            let stor = storage.lock().unwrap();
            let devices = stor.list_devices();
            devices.iter()
                .find(|d| d.addr == remote_addr && !d.public_key.is_empty())
                .map(|d| d.public_key.clone())
        };

        // Noise XX handshake (responder): read → write → read
        let noise = pool.noise_context();
        let session = noise.create_responder(known_pubkey.as_deref())?;

        info!("Noise XX handshake (responder) with {}", remote_addr);

        let msg1 = Self::recv_binary(&mut read).await?;
        session.read_handshake_message(&msg1).await?;

        let resp = session.write_handshake_message(&[]).await?;
        write.send(Message::Binary(resp)).await?;

        let msg3 = Self::recv_binary(&mut read).await?;
        let payload = session.read_handshake_message(&msg3).await?;

        if !session.is_handshake_finished().await {
            anyhow::bail!("Noise handshake did not complete");
        }
        info!("Noise handshake complete with {}", remote_addr);

        let session = Arc::new(session);

        // Parse pairing payload from msg3
        let pairing_info = if !payload.is_empty() {
            serde_json::from_slice::<serde_json::Value>(&payload).ok()
                .and_then(|v| {
                    if v.get("action").and_then(|a| a.as_str()) == Some("pair") {
                        Some(PairingInfo {
                            device_id: v.get("ios_device_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            device_name: v.get("ios_device_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        })
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        // If this is a pairing request, store the device
        if let Some(ref pairing) = pairing_info {
            info!("Pairing request from {} ({})", pairing.device_name, pairing.device_id);

            // Get the remote static public key from the Noise session
            // After handshake, we can extract it from the session's transport state
            // For now, we trust the handshake completed successfully and store a placeholder
            // The actual key verification happens via the Noise protocol itself
            let stor = storage.lock().unwrap();
            let device = Device {
                id: pairing.device_id.clone(),
                name: pairing.device_name.clone(),
                addr: remote_addr.clone(),
                public_key: vec![], // Will be filled from Noise session remote static key
                paired_at: chrono::Utc::now().timestamp(),
                last_seen: Some(chrono::Utc::now().timestamp()),
            };
            if let Err(e) = stor.save_device(&device) {
                error!("Failed to save paired device: {}", e);
            }
        }

        // Set up outgoing message pump: plaintext → encrypt → WS binary
        let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

        let session_out = session.clone();
        tokio::spawn(async move {
            while let Some(plaintext) = out_rx.recv().await {
                match session_out.encrypt(&plaintext).await {
                    Ok(encrypted) => {
                        if let Err(e) = write.send(Message::Binary(encrypted)).await {
                            error!("Failed to send: {}", e);
                            break;
                        }
                    }
                    Err(e) => error!("Encrypt failed: {}", e),
                }
            }
        });

        // Determine peer ID
        let peer_id = if let Some(ref pairing) = pairing_info {
            pairing.device_id.clone()
        } else {
            format!("server-{}", remote_addr)
        };

        let peer_info = super::pool::PeerInfo {
            id: peer_id.clone(),
            name: pairing_info.as_ref().map(|p| p.device_name.clone()).unwrap_or_else(|| "unknown".to_string()),
            addr: remote_addr,
            port: 0,
            public_key: vec![],
        };
        let conn = PeerConnection::new(peer_info);
        conn.set_sender(out_tx.clone()).await;
        let conn_clone = conn.clone();
        pool.add_peer_connection(conn).await;

        // Send pairing confirmation via the channel
        if pairing_info.is_some() {
            let confirm = serde_json::json!({
                "pairing_confirmed": true,
            });
            if let Ok(bytes) = serde_json::to_vec(&confirm) {
                let _ = out_tx.send(bytes).await;
            }
        }

        // Update last_seen for known devices
        {
            let stor = storage.lock().unwrap();
            let _ = DeviceStore::update_last_seen(stor.connection(), &peer_id);
        }

        // Read incoming encrypted messages
        loop {
            let msg = match read.next().await {
                Some(Ok(msg)) => msg,
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                None => break,
            };

            match msg {
                Message::Binary(data) => {
                    let plaintext = match session.decrypt(&data).await {
                        Ok(p) => p,
                        Err(e) => {
                            warn!("Decrypt failed: {}", e);
                            continue;
                        }
                    };
                    match serde_json::from_slice::<serde_json::Value>(&plaintext) {
                        Ok(proto_msg) => {
                            Self::handle_incoming(proto_msg, &pool, &conn_clone, &storage).await;
                        }
                        Err(e) => warn!("Invalid protocol message: {}", e),
                    }
                }
                Message::Close(_) => {
                    info!("Client disconnected");
                    break;
                }
                _ => {}
            }
        }

        // Clean up peer from pool on disconnect
        pool.remove_peer(&peer_id).await;
        Ok(())
    }

    async fn recv_binary<R>(read: &mut R) -> Result<Vec<u8>, anyhow::Error>
    where
        R: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        loop {
            match read.next().await {
                Some(Ok(Message::Binary(data))) => return Ok(data),
                Some(Ok(_)) => continue,
                Some(Err(e)) => anyhow::bail!("WebSocket error during handshake: {}", e),
                None => anyhow::bail!("Connection closed during handshake"),
            }
        }
    }

    async fn handle_incoming(
        msg: serde_json::Value,
        pool: &Arc<ConnectionPool>,
        conn: &PeerConnection,
        storage: &Arc<std::sync::Mutex<Storage>>,
    ) {
        let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match msg_type {
            "ping" => {
                debug!("Received ping");
                let pong = serde_json::json!({
                    "type": "pong",
                    "id": uuid::Uuid::new_v4().to_string(),
                    "timestamp": chrono::Utc::now().timestamp(),
                    "payload": {}
                });
                if let Ok(bytes) = serde_json::to_vec(&pong) {
                    let _ = conn.send_encrypted(&bytes).await;
                }
            }
            "device.hello" => {
                let device_id = msg.get("device_id").and_then(|v| v.as_str())
                    .or_else(|| msg.get("payload").and_then(|p| p.get("device_id")).and_then(|v| v.as_str()))
                    .unwrap_or("unknown");
                let device_name = msg.get("device_name").and_then(|v| v.as_str())
                    .or_else(|| msg.get("payload").and_then(|p| p.get("device_name")).and_then(|v| v.as_str()))
                    .unwrap_or("unknown");
                info!("Device connected: {} ({})", device_name, device_id);

                // Update last_seen
                let stor = storage.lock().unwrap();
                let _ = DeviceStore::update_last_seen(stor.connection(), device_id);
            }
            "clipboard.sync" => {
                info!("Received clipboard sync");
                if let Some(data) = msg.get("payload").and_then(|p| p.get("data")).and_then(|v| v.as_str()) {
                    if let Err(e) = cli_clipboard::set_contents(data.to_string()) {
                        warn!("Failed to set clipboard: {}", e);
                    } else {
                        debug!("Clipboard updated from peer ({} chars)", data.len());
                    }
                }
            }
            "handoff.url" => {
                if let Some(url) = msg.get("payload").and_then(|p| p.get("url")).and_then(|v| v.as_str()) {
                    info!("Received handoff URL: {}", url);
                    if let Err(e) = crate::features::handoff::HandoffManager::handle_incoming_url(url).await {
                        warn!("Failed to open URL: {}", e);
                    }
                }
            }
            "command.request" => {
                if let Some(cmd) = msg.get("payload").and_then(|p| p.get("command")).and_then(|v| v.as_str()) {
                    info!("Received command request: {}", cmd);
                    let result = crate::features::commands::CommandExecutor::execute_local(cmd).await;
                    let response = serde_json::json!({
                        "type": "command.response",
                        "id": uuid::Uuid::new_v4().to_string(),
                        "timestamp": chrono::Utc::now().timestamp(),
                        "payload": {
                            "request_id": msg.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                            "success": result.is_ok(),
                            "output": result.unwrap_or_else(|e| format!("Error: {}", e)),
                        }
                    });
                    if let Ok(bytes) = serde_json::to_vec(&response) {
                        let _ = conn.send_encrypted(&bytes).await;
                    }
                }
            }
            "command.response" => {
                debug!("Received command response from peer");
            }
            "notification" => {
                debug!("Received notification from peer");
            }
            _ => {
                warn!("Unknown message type: {}", msg_type);
            }
        }
    }
}

struct PairingInfo {
    device_id: String,
    device_name: String,
}
