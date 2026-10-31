#!/bin/bash
set -e

# Handoff installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/Nsjminecraft/handoff/main/install.sh | bash

REPO="Nsjminecraft/handoff"
INSTALL_DIR="/usr/local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"
TEMP_DIR=$(mktemp -d)

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

info() {
    echo -e "\033[1;34m[INFO]\033[0m $1"
}

error() {
    echo -e "\033[1;31m[ERROR]\033[0m $1" >&2
    exit 1
}

success() {
    echo -e "\033[1;32m[SUCCESS]\033[0m $1"
}

check_deps() {
    info "Checking dependencies..."

    if ! command -v cargo &> /dev/null; then
        error "Rust/Cargo not found. Install from https://rustup.rs/"
    fi

    if ! command -v git &> /dev/null; then
        error "Git not found. Please install git first."
    fi

    success "All dependencies found"
}

download_release() {
    info "Downloading latest release..."

    LATEST_URL=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep "browser_download_url.*tar.gz" | cut -d '"' -f 4)

    if [ -z "$LATEST_URL" ]; then
        info "No release found, building from source..."
        build_from_source
        return
    fi

    curl -fsSL "$LATEST_URL" -o "$TEMP_DIR/handoff.tar.gz"
    tar -xzf "$TEMP_DIR/handoff.tar.gz" -C "$TEMP_DIR" --strip-components=1

    success "Downloaded latest release"
}

build_from_source() {
    info "Building from source..."

    git clone "https://github.com/$REPO.git" "$TEMP_DIR/handoff"
    cd "$TEMP_DIR/handoff"

    cargo build --release

    success "Build complete"
}

install_binaries() {
    info "Installing binaries to $INSTALL_DIR..."

    if [ -d "$TEMP_DIR/handoff/target/release" ]; then
        # Built from source
        BIN_DIR="$TEMP_DIR/handoff/target/release"
    else
        # Downloaded release
        BIN_DIR="$TEMP_DIR"
    fi

    # Check if binaries exist
    if [ ! -f "$BIN_DIR/handoffd" ]; then
        error "handoffd binary not found. Build may have failed."
    fi

    # Install with sudo if needed
    if [ -w "$INSTALL_DIR" ]; then
        cp "$BIN_DIR/handoffd" "$INSTALL_DIR/"
        cp "$BIN_DIR/handoff" "$INSTALL_DIR/"
        if [ -f "$BIN_DIR/handoff-gui" ]; then
            cp "$BIN_DIR/handoff-gui" "$INSTALL_DIR/"
        fi
    else
        sudo cp "$BIN_DIR/handoffd" "$INSTALL_DIR/"
        sudo cp "$BIN_DIR/handoff" "$INSTALL_DIR/"
        if [ -f "$BIN_DIR/handoff-gui" ]; then
            sudo cp "$BIN_DIR/handoff-gui" "$INSTALL_DIR/"
        fi
    fi

    chmod +x "$INSTALL_DIR/handoffd"
    chmod +x "$INSTALL_DIR/handoff"
    if [ -f "$INSTALL_DIR/handoff-gui" ]; then
        chmod +x "$INSTALL_DIR/handoff-gui"
    fi

    success "Binaries installed"
}

install_desktop_entry() {
    info "Installing desktop entry..."

    DESKTOP_DIR="$HOME/.local/share/applications"
    mkdir -p "$DESKTOP_DIR"

    if [ -f "$TEMP_DIR/handoff/gui/handoff-gui.desktop" ]; then
        cp "$TEMP_DIR/handoff/gui/handoff-gui.desktop" "$DESKTOP_DIR/"
        success "Desktop entry installed"
    else
        info "Desktop entry not found, skipping"
    fi
}

install_systemd_service() {
    info "Installing systemd service..."

    mkdir -p "$SYSTEMD_DIR"

    if [ -f "$TEMP_DIR/handoff/systemd/handoffd.service" ]; then
        cp "$TEMP_DIR/handoff/systemd/handoffd.service" "$SYSTEMD_DIR/"
    else
        # Create service file if not in release
        cat > "$SYSTEMD_DIR/handoffd.service" << 'EOF'
[Unit]
Description=Handoff Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/handoffd
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF
    fi

    systemctl --user daemon-reload
    systemctl --user enable handoffd
    systemctl --user start handoffd

    success "Systemd service installed and started"
}

create_config() {
    info "Creating default configuration..."

    CONFIG_DIR="$HOME/.config/handoff"
    mkdir -p "$CONFIG_DIR"

    if [ ! -f "$CONFIG_DIR/config.toml" ]; then
        cat > "$CONFIG_DIR/config.toml" << 'EOF'
[daemon]
ws_port = 9877
mdns_service = "_handoff._tcp.local."

[clipboard]
history_size = 100
sync_enabled = true

[proximity]
enabled = true
lock_distance = -70
unlock_distance = -60

[camera]
enabled = false
device_index = 0
EOF
        success "Default configuration created at $CONFIG_DIR/config.toml"
    else
        info "Configuration already exists, skipping"
    fi
}

main() {
    echo "==================================="
    echo "  Handoff Installer"
    echo "==================================="
    echo

    check_deps
    download_release
    install_binaries
    install_desktop_entry
    install_systemd_service
    create_config

    echo
    success "Installation complete!"
    echo
    echo "Next steps:"
    echo "  1. Check status: handoff status"
    echo "  2. Pair with iOS: handoff pair"
    echo "  3. Install iOS app from GitHub Releases"
    echo
    echo "For more info: https://github.com/$REPO"
}

main
