use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Application, Color, Command, Element, Length, Theme};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

mod client;
use client::DaemonClient;

#[derive(Debug, Clone)]
enum Message {
    Connected,
    Disconnected,
    DevicesReceived(Vec<DeviceInfo>),
    PairingQrGenerated(String),
    ConnectToDevice(String),
    Error(String),
    Tick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceInfo {
    id: String,
    name: String,
    addr: String,
    port: u16,
    online: bool,
}

struct HandoffApp {
    devices: Vec<DeviceInfo>,
    connected: bool,
    pairing_qr: Option<String>,
    client: Arc<Mutex<Option<DaemonClient>>>,
}

impl HandoffApp {
    fn new() -> (Self, Command<Message>) {
        let client = Arc::new(Mutex::new(None));
        let client_clone = client.clone();

        (
            Self {
                devices: Vec::new(),
                connected: false,
                pairing_qr: None,
                client,
            },
            Command::perform(
                async move {
                    let mut daemon_client = DaemonClient::new("ws://127.0.0.1:9877");
                    if daemon_client.connect().await.is_ok() {
                        let mut guard = client_clone.lock().await;
                        *guard = Some(daemon_client);
                        Message::Connected
                    } else {
                        Message::Disconnected
                    }
                },
                |msg| msg,
            ),
        )
    }

    fn view(&self) -> Element<Message> {
        let status = if self.connected {
            text("Connected to daemon").style(Color::from_rgb(0.0, 0.8, 0.0))
        } else {
            text("Disconnected").style(Color::from_rgb(0.8, 0.0, 0.0))
        };

        let mut content = column![
            status,
            text("Handoff").size(32),
            text("Devices").size(20),
        ]
        .spacing(10)
        .align_items(Alignment::Center);

        if self.devices.is_empty() {
            content = content.push(text("No devices discovered").style(Color::from_rgb(0.5, 0.5, 0.5)));
        } else {
            for device in &self.devices {
                let status_icon = if device.online { "●" } else { "○" };
                let device_row = row![
                    text(status_icon),
                    column![
                        text(&device.name).size(16),
                        text(format!("{}:{}", device.addr, device.port)).size(12),
                    ]
                    .spacing(2),
                    button("Connect").on_press(Message::ConnectToDevice(device.id.clone())),
                ]
                .spacing(10)
                .align_items(Alignment::Center);

                content = content.push(device_row);
            }
        }

        content = content.push(
            button("Generate Pairing QR")
                .on_press(Message::PairingQrGenerated("test".to_string())),
        );

        if let Some(ref qr_data) = self.pairing_qr {
            content = content.push(
                column![
                    text("QR Code (scan with iOS):").size(14),
                    text(qr_data).size(10),
                ]
                .spacing(5),
            );
        }

        container(content)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Connected => {
                self.connected = true;
                Command::none()
            }
            Message::Disconnected => {
                self.connected = false;
                Command::none()
            }
            Message::DevicesReceived(devices) => {
                self.devices = devices;
                Command::none()
            }
            Message::PairingQrGenerated(_) => {
                let qr_data = r#"{"device_id":"linux-pc","device_name":"Linux Desktop","ws_port":"9877","noise_pubkey":"dGVzdA=="}"#;
                self.pairing_qr = Some(qr_data.to_string());
                Command::none()
            }
            Message::ConnectToDevice(device_id) => {
                println!("Connecting to device: {}", device_id);
                Command::none()
            }
            Message::Error(msg) => {
                eprintln!("Error: {}", msg);
                Command::none()
            }
            Message::Tick => {
                Command::none()
            }
        }
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

impl iced::Application for HandoffApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        HandoffApp::new()
    }

    fn title(&self) -> String {
        String::from("Handoff")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        HandoffApp::update(self, message)
    }

    fn view(&self) -> Element<Message> {
        HandoffApp::view(self)
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

fn main() -> iced::Result {
    HandoffApp::run(iced::Settings::default())
}
