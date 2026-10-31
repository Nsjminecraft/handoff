use anyhow::Result;
use libmdns::Responder;
use std::sync::Arc;
use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt};
use tracing::{debug, info, warn};

use crate::connection::pool::ConnectionPool;

pub struct MdnsDiscovery {
    port: u16,
    pool: Arc<ConnectionPool>,
    _responder: Option<Responder>,
}

impl MdnsDiscovery {
    pub async fn new(port: u16, pool: Arc<ConnectionPool>) -> Result<Self> {
        Ok(Self { port, pool, _responder: None })
    }

    pub async fn start(&mut self) -> Result<()> {
        let port = self.port;

        // Register our service
        let (responder, task) = Responder::with_default_handle()?;
        let _svc = responder.register(
            "_handoff._tcp.local.".to_string(),
            "handoff-linux".to_string(),
            port,
            &["version=0.1.0"],
        );
        self._responder = Some(responder);
        info!("Registered mDNS service _handoff._tcp.local. on port {}", port);
        tokio::spawn(task);

        // Browse for peers using avahi-browse
        tokio::spawn(Self::browse_loop(self.pool.clone(), port));

        Ok(())
    }

    async fn browse_loop(pool: Arc<ConnectionPool>, local_port: u16) {
        use crate::connection::client;

        loop {
            // Run avahi-browse in resolve mode to get host/port for each service
            let output = Command::new("avahi-browse")
                .args(["-rtp", "_handoff._tcp"])
                .output()
                .await;

            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // Parse avahi-browse output: lines like
                    // +;wlan0;IPv4;handoff-linux;_handoff._tcp;local;
                    // =;wlan0;IPv4;handoff-linux;_handoff._tcp;local;hostname.local;192.168.1.5;9877;version=0.1.0
                    for line in stdout.lines() {
                        if !line.starts_with('=') {
                            continue;
                        }
                        let parts: Vec<&str> = line.split(';').collect();
                        if parts.len() < 8 {
                            continue;
                        }
                        let name = parts[3];
                        let hostname = parts[6];
                        let addr = parts[7];
                        let port: u16 = match parts.get(8).and_then(|p| p.parse().ok()) {
                            Some(p) => p,
                            None => continue,
                        };

                        // Skip ourselves
                        if port == local_port {
                            continue;
                        }

                        debug!("Discovered peer: {} at {}:{}", name, addr, port);

                        // Check if already connected
                        let peer_ids = pool.peer_ids().await;
                        if peer_ids.iter().any(|id| id.contains(addr)) {
                            continue;
                        }

                        // Connect to peer
                        let peer_info = crate::connection::pool::PeerInfo {
                            id: format!("mdns-{}", addr),
                            name: name.to_string(),
                            addr: addr.to_string(),
                            port,
                            public_key: vec![],
                        };

                        let noise = pool.noise_context().clone();
                        let pool = pool.clone();

                        // We need an incoming message handler — for now just log
                        let (_incoming_tx, mut incoming_rx) = tokio::sync::mpsc::channel::<crate::connection::client::IncomingMessage>(64);
                        tokio::spawn(async move {
                            while let Some(msg) = incoming_rx.recv().await {
                                debug!("Incoming from {}: {:?}", msg.peer_id, msg.message);
                            }
                        });

                        match client::connect_to_peer(&addr, port, noise, peer_info, _incoming_tx).await {
                            Ok(conn) => {
                                pool.add_peer_connection(conn).await;
                                info!("Connected to peer at {}:{}", addr, port);
                            }
                            Err(e) => {
                                warn!("Failed to connect to {}:{}: {}", addr, port, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("avahi-browse failed (may not be installed): {}", e);
                }
            }

            // Re-scan every 10 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }
}
