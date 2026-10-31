use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input, Space};
use iced::{Alignment, Application, Color, Command, Element, Length, Subscription, Theme};
use std::sync::Arc;
use tokio::sync::Mutex;

mod client;
use client::{DaemonClient, DeviceInfo, IpcResponse};

#[derive(Debug, Clone)]
enum Message {
    // Connection
    Connected,
    Disconnected,
    Tick,

    // Devices
    DevicesReceived(Vec<DeviceInfo>),
    RefreshDevices,
    ConnectToDevice(String),
    UnpairDevice(String),

    // Clipboard
    ClipboardReceived(Option<String>),
    ClipboardTextChanged(String),
    SendClipboard,
    ReceiveClipboard,
    ToggleAutoSync,

    // URL Handoff
    UrlTextChanged(String),
    SendUrl,

    // Notifications
    NotifyTitleChanged(String),
    NotifyBodyChanged(String),
    SendNotification,

    // Commands
    CommandTextChanged(String),
    ExecCommand,

    // Pairing
    PairingQrGenerated(String, String),
    GeneratePairingQr,

    // Selection
    SelectedDeviceChanged(Option<String>),
    TabChanged(Tab),

    // Status
    StatusReceived(usize, bool),
    RefreshStatus,

    // Error
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Tab {
    Devices,
    Clipboard,
    Handoff,
    Notifications,
    Commands,
}

struct HandoffApp {
    // Connection
    connected: bool,
    client: Arc<Mutex<Option<DaemonClient>>>,

    // Devices
    devices: Vec<DeviceInfo>,
    selected_device: Option<String>,

    // Status
    peer_count: usize,
    daemon_running: bool,

    // Clipboard
    clipboard_text: String,
    received_clipboard: Option<String>,
    auto_sync: bool,

    // URL Handoff
    url_text: String,

    // Notifications
    notify_title: String,
    notify_body: String,

    // Commands
    command_text: String,

    // Pairing
    pairing_qr: Option<String>,
    pairing_data: Option<String>,

    // UI State
    current_tab: Tab,
    status_message: Option<String>,
}

impl HandoffApp {
    fn new() -> (Self, Command<Message>) {
        let client = Arc::new(Mutex::new(None));
        let client_clone = client.clone();

        (
            Self {
                connected: false,
                client,
                devices: Vec::new(),
                selected_device: None,
                peer_count: 0,
                daemon_running: false,
                clipboard_text: String::new(),
                received_clipboard: None,
                auto_sync: false,
                url_text: String::new(),
                notify_title: String::new(),
                notify_body: String::new(),
                command_text: String::new(),
                pairing_qr: None,
                pairing_data: None,
                current_tab: Tab::Devices,
                status_message: None,
            },
            Command::perform(
                async move {
                    let socket_path = dirs::data_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("~/.local/share"))
                        .join("handoff")
                        .join("handoff.sock");

                    let mut daemon_client = DaemonClient::new(socket_path.to_str().unwrap());
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
            row![
                text("●").style(Color::from_rgb(0.0, 0.8, 0.0)),
                text(format!("Connected ({} peers)", self.peer_count)),
            ]
            .spacing(5)
        } else {
            row![
                text("●").style(Color::from_rgb(0.8, 0.0, 0.0)),
                text("Disconnected"),
            ]
            .spacing(5)
        };

        let tabs = row![
            button("Devices").on_press(Message::TabChanged(Tab::Devices)),
            button("Clipboard").on_press(Message::TabChanged(Tab::Clipboard)),
            button("Handoff").on_press(Message::TabChanged(Tab::Handoff)),
            button("Notifications").on_press(Message::TabChanged(Tab::Notifications)),
            button("Commands").on_press(Message::TabChanged(Tab::Commands)),
        ]
        .spacing(5);

        let mut content = column![
            status,
            text("Handoff").size(32),
            tabs,
            Space::with_height(10),
        ]
        .spacing(10)
        .align_items(Alignment::Center);

        // Tab content
        match self.current_tab {
            Tab::Devices => {
                content = content.push(self.view_devices());
            }
            Tab::Clipboard => {
                content = content.push(self.view_clipboard());
            }
            Tab::Handoff => {
                content = content.push(self.view_handoff());
            }
            Tab::Notifications => {
                content = content.push(self.view_notifications());
            }
            Tab::Commands => {
                content = content.push(self.view_commands());
            }
        }

        if let Some(ref msg) = self.status_message {
            content = content.push(
                text(msg).style(Color::from_rgb(0.0, 0.6, 1.0))
            );
        }

        container(content)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_devices(&self) -> Element<Message> {
        let mut content = column![
            text("Devices").size(24),
            button("Refresh").on_press(Message::RefreshDevices),
        ]
        .spacing(10);

        if self.devices.is_empty() {
            content = content.push(text("No devices discovered").style(Color::from_rgb(0.5, 0.5, 0.5)));
        } else {
            for device in &self.devices {
                let status_icon = if device.connected { "●" } else { "○" };
                let device_row = row![
                    text(status_icon),
                    column![
                        text(&device.name).size(16),
                        text(format!("{} (ID: {})", device.addr, device.id)).size(12),
                    ]
                    .spacing(2),
                    Space::with_width(Length::Fill),
                    button("Unpair").on_press(Message::UnpairDevice(device.id.clone())),
                ]
                .spacing(10)
                .align_items(Alignment::Center);

                content = content.push(device_row);
            }
        }

        content = content.push(Space::with_height(20));
        content = content.push(
            button("Generate Pairing QR").on_press(Message::GeneratePairingQr)
        );

        if let Some(ref qr_data) = self.pairing_qr {
            content = content.push(
                scrollable(
                    column![
                        text("QR Code (scan with iOS):").size(14),
                        text(qr_data).size(10),
                    ]
                    .spacing(5)
                )
                .height(Length::Fixed(200.0))
            );
        }

        content.into()
    }

    fn view_clipboard(&self) -> Element<Message> {
        let mut content = column![
            text("Clipboard Sync").size(24),
        ]
        .spacing(10);

        // Auto sync toggle
        let auto_sync_label = if self.auto_sync { "Auto Sync: ON" } else { "Auto Sync: OFF" };
        content = content.push(
            button(auto_sync_label).on_press(Message::ToggleAutoSync)
        );

        // Manual send
        content = content.push(
            column![
                text("Send to devices:").size(16),
                text_input("Enter text to sync...", &self.clipboard_text)
                    .on_input(Message::ClipboardTextChanged)
                    .on_submit(Message::SendClipboard),
                button("Send Clipboard").on_press(Message::SendClipboard),
            ]
            .spacing(5)
        );

        // Manual receive
        content = content.push(
            column![
                text("Received from devices:").size(16),
                button("Refresh").on_press(Message::ReceiveClipboard),
            ]
            .spacing(5)
        );

        if let Some(ref clipboard_text) = self.received_clipboard {
            content = content.push(
                scrollable(
                    column![
                        text(clipboard_text).size(12),
                    ]
                    .spacing(5)
                )
                .height(Length::Fixed(150.0))
            );
        } else {
            content = content.push(text("No clipboard data received").style(Color::from_rgb(0.5, 0.5, 0.5)));
        }

        content.into()
    }

    fn view_handoff(&self) -> Element<Message> {
        let device_options: Vec<String> = self.devices.iter()
            .filter(|d| d.connected)
            .map(|d| format!("{} ({})", d.name, d.id))
            .collect();

        let mut content = column![
            text("URL Handoff").size(24),
            text("Send a URL to a connected device:").size(16),
        ]
        .spacing(10);

        if device_options.is_empty() {
            content = content.push(text("No connected devices available").style(Color::from_rgb(0.8, 0.0, 0.0)));
        } else {
            content = content.push(
                column![
                    text("Select device:").size(14),
                    pick_list(
                        device_options,
                        self.selected_device.clone(),
                        |s| Message::SelectedDeviceChanged(Some(s))
                    ),
                    text_input("Enter URL to send...", &self.url_text)
                        .on_input(Message::UrlTextChanged)
                        .on_submit(Message::SendUrl),
                    button("Send URL").on_press(Message::SendUrl),
                ]
                .spacing(5)
            );
        }

        content.into()
    }

    fn view_notifications(&self) -> Element<Message> {
        let mut content = column![
            text("Send Notification").size(24),
            text("Send a notification to all connected devices:").size(16),
        ]
        .spacing(10);

        content = content.push(
            column![
                text("Title:").size(14),
                text_input("Notification title...", &self.notify_title)
                    .on_input(Message::NotifyTitleChanged),
                text("Body:").size(14),
                text_input("Notification body...", &self.notify_body)
                    .on_input(Message::NotifyBodyChanged),
                button("Send Notification").on_press(Message::SendNotification),
            ]
            .spacing(5)
        );

        content.into()
    }

    fn view_commands(&self) -> Element<Message> {
        let device_options: Vec<String> = self.devices.iter()
            .filter(|d| d.connected)
            .map(|d| format!("{} ({})", d.name, d.id))
            .collect();

        let mut content = column![
            text("Remote Commands").size(24),
            text("Execute a command on a connected device:").size(16),
        ]
        .spacing(10);

        if device_options.is_empty() {
            content = content.push(text("No connected devices available").style(Color::from_rgb(0.8, 0.0, 0.0)));
        } else {
            content = content.push(
                column![
                    text("Select device:").size(14),
                    pick_list(
                        device_options,
                        self.selected_device.clone(),
                        |s| Message::SelectedDeviceChanged(Some(s))
                    ),
                    text_input("Enter command to execute...", &self.command_text)
                        .on_input(Message::CommandTextChanged)
                        .on_submit(Message::ExecCommand),
                    button("Execute Command").on_press(Message::ExecCommand),
                ]
                .spacing(5)
            );
        }

        content.into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Connected => {
                self.connected = true;
                self.status_message = Some("Connected to daemon".to_string());
                Command::perform(async {}, |_| Message::RefreshStatus)
            }
            Message::Disconnected => {
                self.connected = false;
                self.status_message = Some("Disconnected from daemon".to_string());
                Command::none()
            }
            Message::Tick => {
                if self.auto_sync {
                    let client = self.client.clone();
                    Command::perform(
                        async move {
                            let mut guard = client.lock().await;
                            if let Some(ref mut c) = *guard {
                                match c.get_clipboard().await {
                                    Ok(IpcResponse::Clipboard { text }) => Message::ClipboardReceived(text),
                                    _ => Message::Tick,
                                }
                            } else {
                                Message::Tick
                            }
                        },
                        |msg| msg,
                    )
                } else {
                    Command::none()
                }
            }
            Message::DevicesReceived(devices) => {
                self.devices = devices;
                self.status_message = Some(format!("Found {} devices", self.devices.len()));
                Command::none()
            }
            Message::RefreshDevices => {
                let client = self.client.clone();
                Command::perform(
                    async move {
                        let mut guard = client.lock().await;
                        if let Some(ref mut c) = *guard {
                            match c.get_devices().await {
                                Ok(IpcResponse::Devices { devices }) => Message::DevicesReceived(devices),
                                Ok(IpcResponse::Error { message }) => Message::Error(message),
                                _ => Message::Error("Unexpected response".to_string()),
                            }
                        } else {
                            Message::Error("Not connected".to_string())
                        }
                    },
                    |msg| msg,
                )
            }
            Message::ConnectToDevice(device_id) => {
                println!("Connecting to device: {}", device_id);
                Command::none()
            }
            Message::UnpairDevice(device_id) => {
                let client = self.client.clone();
                Command::perform(
                    async move {
                        let mut guard = client.lock().await;
                        if let Some(ref mut c) = *guard {
                            match c.unpair_device(device_id).await {
                                Ok(IpcResponse::Ok { message }) => Message::Error(format!("OK: {}", message)),
                                Ok(IpcResponse::Error { message }) => Message::Error(message),
                                _ => Message::Error("Unexpected response".to_string()),
                            }
                        } else {
                            Message::Error("Not connected".to_string())
                        }
                    },
                    |msg| msg,
                )
            }
            Message::ClipboardReceived(text) => {
                self.received_clipboard = text;
                Command::none()
            }
            Message::ClipboardTextChanged(text) => {
                self.clipboard_text = text;
                Command::none()
            }
            Message::SendClipboard => {
                let client = self.client.clone();
                let text = self.clipboard_text.clone();
                Command::perform(
                    async move {
                        let mut guard = client.lock().await;
                        if let Some(ref mut c) = *guard {
                            match c.set_clipboard(text).await {
                                Ok(IpcResponse::Ok { message }) => Message::Error(format!("OK: {}", message)),
                                Ok(IpcResponse::Error { message }) => Message::Error(message),
                                _ => Message::Error("Unexpected response".to_string()),
                            }
                        } else {
                            Message::Error("Not connected".to_string())
                        }
                    },
                    |msg| msg,
                )
            }
            Message::ReceiveClipboard => {
                let client = self.client.clone();
                Command::perform(
                    async move {
                        let mut guard = client.lock().await;
                        if let Some(ref mut c) = *guard {
                            match c.get_clipboard().await {
                                Ok(IpcResponse::Clipboard { text }) => Message::ClipboardReceived(text),
                                Ok(IpcResponse::Error { message }) => Message::Error(message),
                                _ => Message::Error("Unexpected response".to_string()),
                            }
                        } else {
                            Message::Error("Not connected".to_string())
                        }
                    },
                    |msg| msg,
                )
            }
            Message::ToggleAutoSync => {
                self.auto_sync = !self.auto_sync;
                self.status_message = Some(format!("Auto sync: {}", if self.auto_sync { "ON" } else { "OFF" }));
                Command::none()
            }
            Message::UrlTextChanged(text) => {
                self.url_text = text;
                Command::none()
            }
            Message::SendUrl => {
                if let Some(ref device_id) = self.selected_device {
                    let client = self.client.clone();
                    let url = self.url_text.clone();
                    let device_id = device_id.clone();
                    Command::perform(
                        async move {
                            let mut guard = client.lock().await;
                            if let Some(ref mut c) = *guard {
                                match c.handoff_url(url, device_id).await {
                                    Ok(IpcResponse::Ok { message }) => Message::Error(format!("OK: {}", message)),
                                    Ok(IpcResponse::Error { message }) => Message::Error(message),
                                    _ => Message::Error("Unexpected response".to_string()),
                                }
                            } else {
                                Message::Error("Not connected".to_string())
                            }
                        },
                        |msg| msg,
                    )
                } else {
                    self.status_message = Some("Please select a device first".to_string());
                    Command::none()
                }
            }
            Message::NotifyTitleChanged(text) => {
                self.notify_title = text;
                Command::none()
            }
            Message::NotifyBodyChanged(text) => {
                self.notify_body = text;
                Command::none()
            }
            Message::SendNotification => {
                let client = self.client.clone();
                let title = self.notify_title.clone();
                let body = self.notify_body.clone();
                Command::perform(
                    async move {
                        let mut guard = client.lock().await;
                        if let Some(ref mut c) = *guard {
                            match c.send_notification(title, body).await {
                                Ok(IpcResponse::Ok { message }) => Message::Error(format!("OK: {}", message)),
                                Ok(IpcResponse::Error { message }) => Message::Error(message),
                                _ => Message::Error("Unexpected response".to_string()),
                            }
                        } else {
                            Message::Error("Not connected".to_string())
                        }
                    },
                    |msg| msg,
                )
            }
            Message::CommandTextChanged(text) => {
                self.command_text = text;
                Command::none()
            }
            Message::ExecCommand => {
                if let Some(ref device_id) = self.selected_device {
                    let client = self.client.clone();
                    let command = self.command_text.clone();
                    let device_id = device_id.clone();
                    Command::perform(
                        async move {
                            let mut guard = client.lock().await;
                            if let Some(ref mut c) = *guard {
                                match c.exec_command(command, device_id).await {
                                    Ok(IpcResponse::Ok { message }) => Message::Error(format!("OK: {}", message)),
                                    Ok(IpcResponse::Error { message }) => Message::Error(message),
                                    _ => Message::Error("Unexpected response".to_string()),
                                }
                            } else {
                                Message::Error("Not connected".to_string())
                            }
                        },
                        |msg| msg,
                    )
                } else {
                    self.status_message = Some("Please select a device first".to_string());
                    Command::none()
                }
            }
            Message::PairingQrGenerated(qr, data) => {
                self.pairing_qr = Some(qr);
                self.pairing_data = Some(data);
                Command::none()
            }
            Message::GeneratePairingQr => {
                let client = self.client.clone();
                Command::perform(
                    async move {
                        let mut guard = client.lock().await;
                        if let Some(ref mut c) = *guard {
                            match c.generate_pair().await {
                                Ok(IpcResponse::Pairing { qr_code, pairing_data }) => {
                                    Message::PairingQrGenerated(qr_code, pairing_data)
                                }
                                Ok(IpcResponse::Error { message }) => Message::Error(message),
                                _ => Message::Error("Unexpected response".to_string()),
                            }
                        } else {
                            Message::Error("Not connected".to_string())
                        }
                    },
                    |msg| msg,
                )
            }
            Message::SelectedDeviceChanged(device) => {
                self.selected_device = device;
                Command::none()
            }
            Message::TabChanged(tab) => {
                self.current_tab = tab;
                Command::none()
            }
            Message::StatusReceived(peer_count, running) => {
                self.peer_count = peer_count;
                self.daemon_running = running;
                Command::none()
            }
            Message::RefreshStatus => {
                let client = self.client.clone();
                Command::perform(
                    async move {
                        let mut guard = client.lock().await;
                        if let Some(ref mut c) = *guard {
                            match c.get_status().await {
                                Ok(IpcResponse::Status { peer_count, running }) => {
                                    Message::StatusReceived(peer_count, running)
                                }
                                Ok(IpcResponse::Error { message }) => Message::Error(message),
                                _ => Message::Error("Unexpected response".to_string()),
                            }
                        } else {
                            Message::Error("Not connected".to_string())
                        }
                    },
                    |msg| msg,
                )
            }
            Message::Error(msg) => {
                self.status_message = Some(msg);
                Command::none()
            }
        }
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_secs(5))
            .map(|_| Message::Tick)
    }
}

impl Application for HandoffApp {
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

    fn subscription(&self) -> Subscription<Message> {
        HandoffApp::subscription(self)
    }
}

fn main() -> iced::Result {
    HandoffApp::run(iced::Settings::default())
}
