mod config;
mod connection;
mod discovery;
mod features;
mod ipc;
mod storage;

use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tracing::info;

use crate::config::Config;
use crate::connection::pool::ConnectionPool;
use crate::discovery::mdns::MdnsDiscovery;
use crate::features::clipboard::ClipboardManager;
use crate::features::notifications::NotificationManager;
use crate::features::handoff::HandoffManager;
use crate::features::commands::CommandExecutor;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("handoffd=info".parse()?),
        )
        .init();

    info!("Starting handoffd");

    // Load config
    let config = Config::load()?;
    info!("Loaded config from {:?}", config.config_path());

    // Initialize storage
    let storage = storage::Storage::new(&config.data_dir())?;
    let storage = Arc::new(std::sync::Mutex::new(storage));

    // Initialize connection pool
    let pool = Arc::new(ConnectionPool::new());

    // Start mDNS discovery
    let mut mdns = MdnsDiscovery::new(config.mdns_port, pool.clone()).await?;
    mdns.start().await?;
    info!("mDNS discovery started on port {}", config.mdns_port);

    // Start WebSocket server
    let ws_server = connection::server::WebSocketServer::new(
        config.ws_port,
        pool.clone(),
        storage.clone(),
    );
    let ws_handle = ws_server.start().await?;
    info!("WebSocket server listening on port {}", config.ws_port);

    // Start clipboard manager
    let clipboard = ClipboardManager::new(pool.clone(), storage.clone());
    let clipboard_handle = clipboard.start().await?;
    info!("Clipboard manager started");

    // Start notification manager
    let notifications = NotificationManager::new(pool.clone());
    let notif_handle = notifications.start_listening().await?;
    info!("Notification manager started");

    // Handoff and command managers (used via IPC)
    let _handoff = HandoffManager::new(pool.clone());
    let _commands = CommandExecutor::new(pool.clone());

    // Start IPC server for CLI
    let ipc_server = ipc::IpcServer::new(config.ipc_socket_path(), pool.clone(), storage.clone(), config.clone());
    let ipc_handle = ipc_server.start().await?;
    info!("IPC server started at {:?}", config.ipc_socket_path());

    info!("handoffd is running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Shutting down...");

    // Cleanup
    ipc_handle.abort();
    notif_handle.abort();
    clipboard_handle.abort();
    ws_handle.abort();

    info!("handoffd stopped");
    Ok(())
}
