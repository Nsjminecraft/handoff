# Handoff

Apple Continuity for Linux + iOS. Universal clipboard, handoff, notifications, and more.

## Architecture

- **handoffd** - Rust daemon (core logic, discovery, connections)
- **handoff** - CLI tool for pairing and control
- **handoff-ios** - Swift/SwiftUI iOS app
- **handoff-ui** - Quickshell widgets for Hyprland
- **handoff-relay** - Go cloud relay server (optional, for cross-network sync)
- **handoff-pam** - PAM module for proximity-based login

## Quick Start

```bash
# Build the daemon and CLI
cargo build --release

# Install the systemd service
cp systemd/handoffd.service ~/.config/systemd/user/
systemctl --user enable --now handoffd

# Check status
handoff status

# Pair with iOS device
handoff pair
```

## Features

- Universal Clipboard (text, images, files)
- Handoff (URLs, app state)
- Notification mirroring
- Remote command execution
- iPhone camera as webcam (v4l2loopback)
- Proximity unlock/lock (BLE + Face ID)
- SMS forwarding

## Development

```bash
# Run daemon in dev mode
cargo run --bin handoffd

# Run CLI
cargo run --bin handoff -- status
```

## License

MIT
