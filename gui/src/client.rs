use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub struct DaemonClient {
    url: String,
    ws: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl DaemonClient {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            ws: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _) = connect_async(&self.url).await?;
        self.ws = Some(ws_stream);
        Ok(())
    }

    pub async fn send(&mut self, msg: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut ws) = self.ws {
            ws.send(Message::Text(msg.to_string())).await?;
        }
        Ok(())
    }

    pub async fn receive(&mut self) -> Option<String> {
        if let Some(ref mut ws) = self.ws {
            while let Some(msg) = ws.next().await {
                if let Ok(Message::Text(text)) = msg {
                    return Some(text);
                }
            }
        }
        None
    }
}
