use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_device_id")]
    pub device_id: String,

    #[serde(default = "default_ws_port")]
    pub ws_port: u16,

    #[serde(default = "default_mdns_port")]
    pub mdns_port: u16,

    #[serde(default = "default_device_name")]
    pub device_name: String,

    #[serde(default = "default_config_dir")]
    config_dir: PathBuf,

    #[serde(default = "default_data_dir")]
    data_dir: PathBuf,

    #[serde(default)]
    pub clipboard: ClipboardConfig,

    #[serde(default)]
    pub proximity: ProximityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClipboardConfig {
    #[serde(default = "default_clipboard_history_size")]
    pub history_size: usize,

    #[serde(default = "default_sync_images")]
    pub sync_images: bool,

    #[serde(default = "default_sync_files")]
    pub sync_files: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProximityConfig {
    #[serde(default = "default_lock_rssi")]
    pub lock_rssi: i16,

    #[serde(default = "default_unlock_rssi")]
    pub unlock_rssi: i16,

    #[serde(default = "default_lock_timeout_secs")]
    pub lock_timeout_secs: u64,
}

impl Default for ProximityConfig {
    fn default() -> Self {
        Self {
            lock_rssi: default_lock_rssi(),
            unlock_rssi: default_unlock_rssi(),
            lock_timeout_secs: default_lock_timeout_secs(),
        }
    }
}

fn default_device_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn default_ws_port() -> u16 {
    9877
}

fn default_mdns_port() -> u16 {
    9876
}

fn default_device_name() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "handoff-linux".to_string())
}

fn default_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("handoff")
}

fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("handoff")
}

fn default_clipboard_history_size() -> usize {
    100
}

fn default_sync_images() -> bool {
    true
}

fn default_sync_files() -> bool {
    true
}

fn default_lock_rssi() -> i16 {
    -75
}

fn default_unlock_rssi() -> i16 {
    -60
}

fn default_lock_timeout_secs() -> u64 {
    30
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::default_path();

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            let mut config: Config = toml::from_str(&contents)?;
            config.config_dir = default_config_dir();
            config.data_dir = default_data_dir();
            Ok(config)
        } else {
            // Create default config
            let config = Config {
                device_id: default_device_id(),
                ws_port: default_ws_port(),
                mdns_port: default_mdns_port(),
                device_name: default_device_name(),
                config_dir: default_config_dir(),
                data_dir: default_data_dir(),
                clipboard: ClipboardConfig {
                    history_size: default_clipboard_history_size(),
                    sync_images: default_sync_images(),
                    sync_files: default_sync_files(),
                },
                proximity: ProximityConfig::default(),
            };

            // Save default config
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::default_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, contents)?;
        Ok(())
    }

    fn default_path() -> PathBuf {
        default_config_dir().join("config.toml")
    }

    pub fn config_path(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.data_dir.clone()
    }

    pub fn ipc_socket_path(&self) -> PathBuf {
        self.data_dir.join("handoff.sock")
    }
}
