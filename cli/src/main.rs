use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Parser)]
#[command(name = "handoff", about = "Handoff CLI - control your continuity daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Show daemon status and connected peers
    Status,
    /// Pair with a new device
    Pair,
    /// Get clipboard content
    ClipboardGet,
    /// Set clipboard content
    ClipboardSet {
        /// Text to set
        text: String,
    },
    /// Send a notification
    Notify {
        /// Notification title
        title: String,
        /// Notification body
        body: String,
    },
    /// List paired devices
    Devices,
    /// Unpair a device
    Unpair {
        /// Device ID
        device_id: String,
    },
    /// Send a URL to open on another device
    Handoff {
        /// URL to open
        url: String,
        /// Target device ID
        device_id: String,
    },
    /// Execute a command on a remote device
    Exec {
        /// Command to run
        command: String,
        /// Target device ID
        device_id: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum IpcRequest {
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
    #[serde(rename = "pair")]
    Pair,
    #[serde(rename = "unpair")]
    Unpair { device_id: String },
    #[serde(rename = "handoff.url")]
    HandoffUrl { url: String, device_id: String },
    #[serde(rename = "command.exec")]
    CommandExec { command: String, device_id: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum IpcResponse {
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

#[derive(Debug, Deserialize)]
struct DeviceInfo {
    id: String,
    name: String,
    addr: String,
    connected: bool,
}

fn socket_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("handoff")
        .join("handoff.sock")
}

async fn send_request(request: IpcRequest) -> Result<IpcResponse> {
    let path = socket_path();
    let stream = UnixStream::connect(&path).await.map_err(|e| {
        anyhow::anyhow!("Cannot connect to daemon at {:?}: {}. Is handoffd running?", path, e)
    })?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut request_json = serde_json::to_string(&request)?;
    request_json.push('\n');
    writer.write_all(request_json.as_bytes()).await?;
    writer.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: IpcResponse = serde_json::from_str(&line)?;
    Ok(response)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => {
            match send_request(IpcRequest::Status).await {
                Ok(IpcResponse::Status { peer_count, running }) => {
                    println!("Daemon: {}", if running { "running" } else { "stopped" });
                    println!("Connected peers: {}", peer_count);
                }
                Ok(IpcResponse::Error { message }) => {
                    eprintln!("Error: {}", message);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("Unexpected response from daemon");
                    std::process::exit(1);
                }
            }
        }
        Commands::Devices => {
            match send_request(IpcRequest::Devices).await {
                Ok(IpcResponse::Devices { devices }) => {
                    if devices.is_empty() {
                        println!("No paired devices.");
                    } else {
                        println!("Paired devices:");
                        for d in devices {
                            let status = if d.connected { "connected" } else { "offline" };
                            println!("  {} ({}) - {} [{}]", d.name, d.id, d.addr, status);
                        }
                    }
                }
                Ok(IpcResponse::Error { message }) => {
                    eprintln!("Error: {}", message);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("Unexpected response from daemon");
                    std::process::exit(1);
                }
            }
        }
        Commands::ClipboardGet => {
            match send_request(IpcRequest::ClipboardGet).await {
                Ok(IpcResponse::Clipboard { text }) => {
                    match text {
                        Some(t) => println!("{}", t),
                        None => println!("(clipboard empty)"),
                    }
                }
                Ok(IpcResponse::Error { message }) => {
                    eprintln!("Error: {}", message);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("Unexpected response from daemon");
                    std::process::exit(1);
                }
            }
        }
        Commands::ClipboardSet { text } => {
            match send_request(IpcRequest::ClipboardSet { text }).await {
                Ok(IpcResponse::Ok { message }) => {
                    println!("{}", message);
                }
                Ok(IpcResponse::Error { message }) => {
                    eprintln!("Error: {}", message);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("Unexpected response from daemon");
                    std::process::exit(1);
                }
            }
        }
        Commands::Notify { title, body } => {
            match send_request(IpcRequest::Notify { title, body }).await {
                Ok(IpcResponse::Ok { message }) => {
                    println!("{}", message);
                }
                Ok(IpcResponse::Error { message }) => {
                    eprintln!("Error: {}", message);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("Unexpected response from daemon");
                    std::process::exit(1);
                }
            }
        }
        Commands::Pair => {
            match send_request(IpcRequest::Pair).await {
                Ok(IpcResponse::Pairing { qr_code, pairing_data }) => {
                    println!("Scan this QR code with the Handoff iOS app:\n");
                    println!("{}", qr_code);
                    println!("\nOr share this pairing data manually:");
                    println!("{}", pairing_data);
                }
                Ok(IpcResponse::Error { message }) => {
                    eprintln!("Error: {}", message);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("Unexpected response from daemon");
                    std::process::exit(1);
                }
            }
        }
        Commands::Unpair { device_id } => {
            match send_request(IpcRequest::Unpair { device_id }).await {
                Ok(IpcResponse::Ok { message }) => {
                    println!("{}", message);
                }
                Ok(IpcResponse::Error { message }) => {
                    eprintln!("Error: {}", message);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
                _ => {
                    eprintln!("Unexpected response from daemon");
                    std::process::exit(1);
                }
            }
        }
        Commands::Handoff { url, device_id } => {
            match send_request(IpcRequest::HandoffUrl { url, device_id }).await {
                Ok(IpcResponse::Ok { message }) => println!("{}", message),
                Ok(IpcResponse::Error { message }) => { eprintln!("Error: {}", message); std::process::exit(1); }
                Err(e) => { eprintln!("{}", e); std::process::exit(1); }
                _ => { eprintln!("Unexpected response"); std::process::exit(1); }
            }
        }
        Commands::Exec { command, device_id } => {
            match send_request(IpcRequest::CommandExec { command, device_id }).await {
                Ok(IpcResponse::Ok { message }) => println!("{}", message),
                Ok(IpcResponse::Error { message }) => { eprintln!("Error: {}", message); std::process::exit(1); }
                Err(e) => { eprintln!("{}", e); std::process::exit(1); }
                _ => { eprintln!("Unexpected response"); std::process::exit(1); }
            }
        }
    }

    Ok(())
}
