use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::connection::client::PeerConnection;
use crate::connection::noise::NoiseContext;

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub id: String,
    pub name: String,
    pub addr: String,
    pub port: u16,
    pub public_key: Vec<u8>,
}

pub struct ConnectionPool {
    connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
    noise: Arc<NoiseContext>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            noise: Arc::new(NoiseContext::new()),
        }
    }

    pub async fn add_peer_connection(&self, conn: PeerConnection) {
        let peer_id = conn.peer_info().id.clone();
        let peer_name = conn.peer_info().name.clone();
        let mut conns = self.connections.write().await;
        info!("Adding peer connection: {} ({})", peer_name, peer_id);
        conns.insert(peer_id, conn);
    }

    pub async fn remove_peer(&self, peer_id: &str) {
        let mut conns = self.connections.write().await;
        if conns.remove(peer_id).is_some() {
            info!("Removed peer {}", peer_id);
        }
    }

    pub async fn get_peer(&self, peer_id: &str) -> Option<PeerConnection> {
        let conns = self.connections.read().await;
        conns.get(peer_id).cloned()
    }

    pub async fn broadcast(&self, message: &[u8], exclude: Option<&str>) {
        let conns: Vec<PeerConnection> = {
            let conns = self.connections.read().await;
            conns.iter()
                .filter(|(id, _)| exclude != Some(id.as_str()))
                .map(|(_, conn)| conn.clone())
                .collect()
        };
        for conn in conns {
            if let Err(e) = conn.send_encrypted(message).await {
                tracing::warn!("Failed to send to peer: {}", e);
            }
        }
    }

    pub async fn peer_count(&self) -> usize {
        let conns = self.connections.read().await;
        conns.len()
    }

    pub async fn peer_ids(&self) -> Vec<String> {
        let conns = self.connections.read().await;
        conns.keys().cloned().collect()
    }

    pub fn noise_context(&self) -> &Arc<NoiseContext> {
        &self.noise
    }
}
