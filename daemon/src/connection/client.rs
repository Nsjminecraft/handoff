use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{StreamExt, SinkExt};
use tracing::{debug, error, info, warn};

use super::noise::{NoiseContext, NoiseSession};
use super::pool::PeerInfo;

#[derive(Clone)]
pub struct PeerConnection {
    peer: PeerInfo,
    outgoing: Arc<Mutex<Option<tokio::sync::mpsc::Sender<Vec<u8>>>>>,
}

impl PeerConnection {
    pub fn new(peer: PeerInfo) -> Self {
        Self {
            peer,
            outgoing: Arc::new(Mutex::new(None)),
        }
    }

    pub fn peer_info(&self) -> &PeerInfo {
        &self.peer
    }

    pub async fn set_sender(&self, tx: tokio::sync::mpsc::Sender<Vec<u8>>) {
        let mut outgoing = self.outgoing.lock().await;
        *outgoing = Some(tx);
    }

    pub async fn send_encrypted(&self, data: &[u8]) -> Result<()> {
        let tx = {
            let outgoing = self.outgoing.lock().await;
            outgoing.clone()
        };
        if let Some(tx) = tx {
            tx.send(data.to_vec()).await?;
            Ok(())
        } else {
            anyhow::bail!("No active connection to peer {}", self.peer.id)
        }
    }

    pub fn is_connected(&self) -> bool {
        // Check if sender channel exists and is not closed
        // Simplified — we rely on the outgoing sender being set
        true
    }
}

/// Connect to a remote peer as Noise initiator.
/// Performs the XX handshake, then spawns a reader task.
/// Returns a PeerConnection wired up for sending.
pub async fn connect_to_peer(
    addr: &str,
    port: u16,
    noise: Arc<NoiseContext>,
    peer_info: PeerInfo,
    incoming_tx: tokio::sync::mpsc::Sender<IncomingMessage>,
) -> Result<PeerConnection> {
    let url = format!("ws://{}:{}", addr, port);
    info!("Connecting to peer at {}", url);

    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await?;
    let (mut write, mut read) = ws_stream.split();

    // Perform Noise XX handshake (initiator side)
    // XX pattern initiator: write → read → write
    let session = noise.create_initiator(None)?;

    info!("Starting Noise XX handshake (initiator)");

    // Step 1: send our `e`
    let msg1 = session.write_handshake_message(&[]).await?;
    write.send(Message::Binary(msg1)).await?;

    // Step 2: receive responder's `e, ee, s, es`
    let msg2 = recv_binary(&mut read).await?;
    session.read_handshake_message(&msg2).await?;

    // Step 3: send our `s, se` — handshake complete
    let msg3 = session.write_handshake_message(&[]).await?;
    write.send(Message::Binary(msg3)).await?;

    if !session.is_handshake_finished().await {
        anyhow::bail!("Noise handshake did not complete");
    }
    info!("Noise handshake complete with peer at {}", url);

    let session = Arc::new(session);
    let conn = PeerConnection::new(peer_info);

    // Spawn outgoing message pump: takes plaintext JSON, encrypts, sends via WS
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
    conn.set_sender(out_tx).await;

    let session_out = session.clone();
    tokio::spawn(async move {
        while let Some(plaintext) = out_rx.recv().await {
            match session_out.encrypt(&plaintext).await {
                Ok(encrypted) => {
                    if let Err(e) = write.send(Message::Binary(encrypted)).await {
                        error!("Failed to send encrypted message: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Encryption failed: {}", e);
                }
            }
        }
    });

    // Spawn incoming message reader: reads encrypted WS messages, decrypts, dispatches
    let peer_id = conn.peer_info().id.clone();
    tokio::spawn(async move {
        loop {
            match read.next().await {
                Some(Ok(Message::Binary(data))) => {
                    match session.decrypt(&data).await {
                        Ok(plaintext) => {
                            match serde_json::from_slice::<serde_json::Value>(&plaintext) {
                                Ok(msg) => {
                                    let _ = incoming_tx.send(IncomingMessage {
                                        peer_id: peer_id.clone(),
                                        message: msg,
                                    }).await;
                                }
                                Err(e) => warn!("Invalid JSON from peer: {}", e),
                            }
                        }
                        Err(e) => warn!("Decrypt failed: {}", e),
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    info!("Peer {} disconnected", peer_id);
                    break;
                }
                Some(Ok(_)) => {} // ignore text/ping/pong
                Some(Err(e)) => {
                    error!("WebSocket error from peer {}: {}", peer_id, e);
                    break;
                }
                None => break,
            }
        }
    });

    // Send device.hello to identify ourselves
    let hello = serde_json::json!({
        "type": "device.hello",
        "id": uuid::Uuid::new_v4().to_string(),
        "timestamp": chrono::Utc::now().timestamp(),
        "payload": {
            "device_id": uuid::Uuid::new_v4().to_string(),
            "device_name": hostname::get().ok().and_then(|h| h.into_string().ok()).unwrap_or_else(|| "handoff-linux".to_string()),
        }
    });
    let hello_bytes = serde_json::to_vec(&hello)?;
    conn.send_encrypted(&hello_bytes).await?;

    Ok(conn)
}

async fn recv_binary<R>(read: &mut R) -> Result<Vec<u8>>
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

#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub peer_id: String,
    pub message: serde_json::Value,
}
