# Handoff

Apple Continuity features for Linux and iOS. Seamlessly transfer clipboard content, URLs, notifications, and more between your Linux desktop and iOS devices.

## Features

- **Clipboard Sync** - Copy on one device, paste on another
- **URL Handoff** - Open URLs from one device on another
- **Notification Bridge** - Forward notifications between devices
- **iPhone as Webcam** - Use your iPhone camera on Linux via v4l2loopback
- **Proximity Lock/Unlock** - Auto-lock when phone leaves, unlock when it returns
- **Remote Commands** - Execute shell commands on Linux from iOS
- **SMS Forwarding** - Receive iPhone SMS notifications on Linux
- **E2E Encryption** - All communication encrypted with Noise Protocol

## Architecture

```
┌─────────────┐         ┌─────────────────┐         ┌──────────────┐
│  handoff-ios │◄──WS───►│    handoffd      │◄──WS───►│ handoff-relay │
│  (Swift)     │  E2EE   │    (Rust)        │  E2EE   │ (Go, cloud)  │
└─────────────┘         └───────┬──────────┘         └──────────────┘
                                │ IPC (Unix socket)
                    ┌───────────┼───────────┐
                    │           │           │
             ┌──────▼──┐  ┌────▼────┐  ┌───▼──────┐
             │handoff   │  │handoff  │  │handoff   │
             │-gui      │  │-cli     │  │-pam      │
             │(iced)    │  │(Rust)   │  │(Rust)    │
             └──────────┘  └─────────┘  └──────────┘
             Desktop       Terminal     Linux login
             GUI           control      proximity auth
```

### Components

| Component | Language | Purpose |
|-----------|----------|---------|
| `handoffd` | Rust | Core daemon. Discovery, connections, feature logic |
| `handoff-ios` | Swift/SwiftUI | iOS app with camera, Face ID, BLE, notifications |
| `handoff-gui` | Rust (iced) | Desktop GUI for device management |
| `handoff-cli` | Rust | Terminal CLI for pairing, config, testing |
| `handoff-relay` | Go | Cloud relay server for cross-network sync |
| `handoff-pam` | Rust | PAM module for proximity-based login |
| `handoff-ui` | QML/Quickshell | Hyprland widgets (status, notifications, clipboard) |

## Installation

### Quick Install (Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/Nsjminecraft/handoff/main/install.sh | bash
```

This installs:
- `handoffd` daemon (systemd user service)
- `handoff` CLI tool
- `handoff-gui` desktop app (optional, requires GUI)

### Manual Installation

#### Prerequisites

**Linux:**
- Rust (1.70+)
- Cargo
- systemd (for daemon service)
- Optional: `v4l2loopback-dkms` (for camera feature), `wl-clipboard` (for Wayland clipboard)

**iOS:**
- Xcode 15+ (or `xtool` for Linux builds)
- iOS 16+ device
- Apple Developer account (free tier works for personal use)

#### Build from Source

```bash
# Clone the repository
git clone https://github.com/Nsjminecraft/handoff.git
cd handoff

# Build Linux components
cargo build --release

# Install binaries
sudo cp target/release/handoffd /usr/local/bin/
sudo cp target/release/handoff /usr/local/bin/
sudo cp target/release/handoff-gui /usr/local/bin/  # Optional

# Install systemd service
mkdir -p ~/.config/systemd/user/
cp systemd/handoffd.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable handoffd
systemctl --user start handoffd
```

#### Build iOS App

**On macOS (Xcode):**
```bash
cd ios
# Open in Xcode and configure signing
open HandoffiOS.xcodeproj
# Or build from command line
xcodebuild -scheme HandoffiOS -destination 'generic/platform=iOS'
```

**On Linux (xtool):**
```bash
# Install xtool first
curl -fsSL https://github.com/nicklockwood/XcodeBuild/releases/latest/download/install.sh | bash

# Build IPA
cd ios
xtool dev build --ipa

# Install to connected iOS device
# Connect your iPhone via USB first
xtool install HandoffiOS.ipa
```

**iOS Device Setup:**
1. Connect iPhone via USB
2. Enable Developer Mode: Settings → Privacy & Security → Developer Mode
3. Trust the development certificate: Settings → General → VPN & Device Management
4. Run: `xtool install HandoffiOS.ipa` (Linux) or install via Xcode (macOS)
5. Launch Handoff app on iPhone

## Usage

### 1. Start the Daemon

```bash
systemctl --user start handoffd
```

### 2. Pair Devices

**Linux (generate QR code):**
```bash
handoff pair
```

**iOS:**
1. Open Handoff app
2. Tap "Scan QR Code"
3. Scan the QR code displayed on Linux
4. Verify the fingerprint matches
5. Tap "Pair"

### 3. Check Status

```bash
handoff status
```

### 4. Use Features

**Clipboard:**
```bash
# Copy to clipboard (syncs to iOS)
echo "Hello from Linux" | handoff clipboard set

# Get clipboard from iOS
handoff clipboard get
```

**Notifications:**
```bash
# Send notification to iOS
handoff notify "Test notification"
```

**URL Handoff:**
```bash
# Open URL on iOS
handoff open https://example.com
```

### 5. Hyprland Widgets (Optional)

```bash
# Start Quickshell with Handoff widgets
quickshell --config ui/shell.conf
```

## Configuration

Edit `~/.config/handoff/config.toml`:

```toml
[daemon]
ws_port = 9877
mdns_service = "_handoff._tcp.local."

[clipboard]
history_size = 100
sync_enabled = true

[proximity]
enabled = true
lock_distance = -70  # RSSI threshold
unlock_distance = -60

[camera]
enabled = false
device_index = 0
```

## Development

### Project Structure

```
handoff/
├── daemon/          # Core daemon (Rust)
├── cli/             # CLI tool (Rust)
├── gui/             # Desktop GUI (Rust + iced)
├── ios/             # iOS app (Swift)
├── relay/           # Cloud relay (Go)
├── pam/             # PAM module (Rust)
├── ui/              # Hyprland widgets (QML)
├── systemd/         # Systemd service files
└── docs/            # Protocol documentation
```

### Build All Components

```bash
# Rust components
cargo build --workspace

# iOS (requires xtool on Linux or Xcode on macOS)
cd ios && xtool dev build

# Go relay
cd relay && go build
```

### Run Tests

```bash
cargo test --workspace
```

## Protocol

Communication uses WebSocket with Noise Protocol XX handshake for E2E encryption. Messages are JSON with 4-byte length prefix.

See [docs/protocol.md](docs/protocol.md) for full specification.

## Roadmap

- [x] Clipboard sync
- [x] URL handoff
- [x] Notification bridge
- [x] iPhone camera as webcam
- [x] Proximity lock/unlock
- [x] SMS forwarding
- [x] Remote commands
- [x] PAM integration
- [ ] Screen sharing (xdg-desktop-portal)
- [ ] iMessage bridge (requires Mac relay)
- [ ] Multi-device support
- [ ] Cloud relay deployment guide

## Contributing

Contributions welcome! Please open an issue or PR.

## License

MIT

## Credits

Built with:
- [iced](https://github.com/iced-rs/iced) - Rust GUI framework
- [snow](https://github.com/mcginty/snow) - Noise Protocol implementation
- [tokio](https://github.com/tokio-rs/tokio) - Async runtime
- [bluer](https://github.com/bluez/bluer) - Bluetooth library
- [xtool](https://github.com/nicklockwood/XcodeBuild) - iOS builds on Linux
