use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub addr: String,
    pub connected: bool,
}

pub struct DaemonClient {
    socket_path: String,
    stream: Option<UnixStream>,
}

impl DaemonClient {
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.to_string(),
            stream: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        self.stream = Some(stream);
        Ok(())
    }

    pub async fn send_request(&mut self, request: IpcRequest) -> Result<IpcResponse, Box<dyn std::error::Error>> {
        if self.stream.is_none() {
            return Err("Not connected".into());
        }

        let request_json = serde_json::to_string(&request)?;
        let stream = self.stream.as_mut().ok_or("Not connected")?;

        // Send request
        let mut request_bytes = request_json.into_bytes();
        request_bytes.push(b'\n');
        stream.write_all(&request_bytes).await?;
        stream.flush().await?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let response: IpcResponse = serde_json::from_str(&response_line)?;
        Ok(response)
    }

    pub async fn get_status(&mut self) -> Result<IpcResponse, Box<dyn std::error::Error>> {
        self.send_request(IpcRequest::Status).await
    }

    pub async fn get_devices(&mut self) -> Result<IpcResponse, Box<dyn std::error::Error>> {
        self.send_request(IpcRequest::Devices).await
    }

    pub async fn get_clipboard(&mut self) -> Result<IpcResponse, Box<dyn std::error::Error>> {
        self.send_request(IpcRequest::ClipboardGet).await
    }

    pub async fn set_clipboard(&mut self, text: String) -> Result<IpcResponse, Box<dyn std::error::Error>> {
        self.send_request(IpcRequest::ClipboardSet { text }).await
    }

    pub async fn send_notification(&mut self, title: String, body: String) -> Result<IpcResponse, Box<dyn std::error::Error>> {
        self.send_request(IpcRequest::Notify { title, body }).await
    }

    pub async fn handoff_url(&mut self, url: String, device_id: String) -> Result<IpcResponse, Box<dyn std::error::Error>> {
        self.send_request(IpcRequest::HandoffUrl { url, device_id }).await
    }

    pub async fn exec_command(&mut self, command: String, device_id: String) -> Result<IpcResponse, Box<dyn std::error::Error>> {
        self.send_request(IpcRequest::CommandExec { command, device_id }).await
    }

    pub async fn generate_pair(&mut self) -> Result<IpcResponse, Box<dyn std::error::Error>> {
        self.send_request(IpcRequest::Pair).await
    }

    pub async fn unpair_device(&mut self, device_id: String) -> Result<IpcResponse, Box<dyn std::error::Error>> {
        self.send_request(IpcRequest::Unpair { device_id }).await
    }
}
