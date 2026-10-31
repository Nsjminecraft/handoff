use anyhow::Result;
use snow::{Builder, HandshakeState, TransportState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_SHA256";

fn key_path() -> std::path::PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("handoff");
    std::fs::create_dir_all(&config_dir).ok();
    config_dir.join("noise_key.bin")
}

pub struct NoiseContext {
    local_key: snow::Keypair,
}

impl NoiseContext {
    pub fn new() -> Self {
        let path = key_path();
        let builder = Builder::new(NOISE_PATTERN.parse().unwrap());

        let local_key = if let Ok(data) = std::fs::read(&path) {
            if data.len() == 64 {
                let mut private = vec![0u8; 32];
                private.copy_from_slice(&data[..32]);
                let mut public = vec![0u8; 32];
                public.copy_from_slice(&data[32..]);
                info!("Loaded persisted Noise keypair from {:?}", path);
                snow::Keypair { private, public }
            } else {
                warn!("Invalid key file size at {:?}, regenerating", path);
                let key = builder.generate_keypair().unwrap();
                let mut file_data = Vec::with_capacity(64);
                file_data.extend_from_slice(&key.private);
                file_data.extend_from_slice(&key.public);
                std::fs::write(&path, &file_data).ok();
                key
            }
        } else {
            let key = builder.generate_keypair().unwrap();
            let mut file_data = Vec::with_capacity(64);
            file_data.extend_from_slice(&key.private);
            file_data.extend_from_slice(&key.public);
            if std::fs::write(&path, &file_data).is_ok() {
                info!("Generated and persisted Noise keypair to {:?}", path);
            }
            key
        };

        debug!("Noise public key: {:?}", local_key.public);
        Self { local_key }
    }

    pub fn public_key(&self) -> &[u8] {
        &self.local_key.public
    }

    pub fn create_responder(&self, remote_pubkey: Option<&[u8]>) -> Result<NoiseSession> {
        let builder = Builder::new(NOISE_PATTERN.parse().unwrap());
        let builder = builder.local_private_key(&self.local_key.private);

        let handshake = if let Some(remote) = remote_pubkey {
            builder.remote_public_key(remote).build_responder()?
        } else {
            builder.build_responder()?
        };

        Ok(NoiseSession {
            state: Arc::new(Mutex::new(NoiseState::Handshake(Some(handshake)))),
        })
    }

    pub fn create_initiator(&self, remote_pubkey: Option<&[u8]>) -> Result<NoiseSession> {
        let builder = Builder::new(NOISE_PATTERN.parse().unwrap());
        let builder = builder.local_private_key(&self.local_key.private);

        let handshake = if let Some(remote) = remote_pubkey {
            builder.remote_public_key(remote).build_initiator()?
        } else {
            builder.build_initiator()?
        };

        Ok(NoiseSession {
            state: Arc::new(Mutex::new(NoiseState::Handshake(Some(handshake)))),
        })
    }
}

pub struct NoiseSession {
    state: Arc<Mutex<NoiseState>>,
}

enum NoiseState {
    Handshake(Option<HandshakeState>),
    Transport(TransportState),
}

impl NoiseSession {
    /// Read a handshake message from the peer. Returns decrypted payload (if any).
    pub async fn read_handshake_message(&self, message: &[u8]) -> Result<Vec<u8>> {
        let mut state = self.state.lock().await;
        let hs = match &mut *state {
            NoiseState::Handshake(Some(hs)) => hs,
            NoiseState::Handshake(None) => anyhow::bail!("Handshake state consumed"),
            NoiseState::Transport(_) => anyhow::bail!("Handshake already complete"),
        };

        let mut buf = vec![0u8; 65535];
        let len = hs.read_message(message, &mut buf)?;
        buf.truncate(len);

        // Check if handshake is now complete after this read
        if hs.is_handshake_finished() {
            debug!("Handshake complete after read, switching to transport mode");
            if let NoiseState::Handshake(hs_opt) = &mut *state {
                if let Some(hs) = hs_opt.take() {
                    let transport = hs.into_transport_mode()?;
                    *state = NoiseState::Transport(transport);
                }
            }
        }

        Ok(buf)
    }

    /// Write a handshake message to send to the peer.
    pub async fn write_handshake_message(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut state = self.state.lock().await;
        let hs = match &mut *state {
            NoiseState::Handshake(Some(hs)) => hs,
            NoiseState::Handshake(None) => anyhow::bail!("Handshake state consumed"),
            NoiseState::Transport(_) => anyhow::bail!("Handshake already complete"),
        };

        let mut buf = vec![0u8; 65535];
        let len = hs.write_message(payload, &mut buf)?;
        buf.truncate(len);

        // Check if handshake is now complete after this write
        if hs.is_handshake_finished() {
            debug!("Handshake complete after write, switching to transport mode");
            if let NoiseState::Handshake(hs_opt) = &mut *state {
                if let Some(hs) = hs_opt.take() {
                    let transport = hs.into_transport_mode()?;
                    *state = NoiseState::Transport(transport);
                }
            }
        }

        Ok(buf)
    }

    pub async fn is_handshake_finished(&self) -> bool {
        let state = self.state.lock().await;
        matches!(&*state, NoiseState::Transport(_))
    }

    /// Encrypt a message (only valid after handshake is complete)
    pub async fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let mut state = self.state.lock().await;
        match &mut *state {
            NoiseState::Transport(t) => {
                let mut buf = vec![0u8; plaintext.len() + 16];
                let len = t.write_message(plaintext, &mut buf)?;
                buf.truncate(len);
                Ok(buf)
            }
            NoiseState::Handshake(_) => {
                anyhow::bail!("Cannot encrypt before handshake is complete");
            }
        }
    }

    /// Decrypt a message (only valid after handshake is complete)
    pub async fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let mut state = self.state.lock().await;
        match &mut *state {
            NoiseState::Transport(t) => {
                let mut buf = vec![0u8; ciphertext.len()];
                let len = t.read_message(ciphertext, &mut buf)?;
                buf.truncate(len);
                Ok(buf)
            }
            NoiseState::Handshake(_) => {
                anyhow::bail!("Cannot decrypt before handshake is complete");
            }
        }
    }
}
