# iOS Deployment Guide

## Prerequisites

- iPhone with iOS 15+
- USB cable
- Linux or macOS machine
- `xtool` (Linux) or Xcode (macOS)

## Step 1: Enable Developer Mode on iPhone

1. Open **Settings** → **Privacy & Security**
2. Scroll down to **Developer Mode**
3. Toggle it on
4. Restart your iPhone when prompted

## Step 2: Build the iOS App

**On Linux:**
```bash
# Install xtool if you haven't
curl -fsSL https://github.com/nicklockwood/XcodeBuild/releases/latest/download/install.sh | bash

# Build the IPA
cd ~/Projects/handoff/ios
xtool dev build --ipa
```

**On macOS:**
```bash
cd ~/Projects/handoff/ios
# Open in Xcode, configure signing, then build
open HandoffiOS.xcodeproj
# Or from command line:
xcodebuild -scheme HandoffiOS -destination 'generic/platform=iOS'
```

## Step 3: Install on iPhone

1. **Connect your iPhone via USB**
2. **Trust the computer** if prompted on iPhone
3. **Install the app:**

**Linux:**
```bash
cd ~/Projects/handoff/ios
xtool install HandoffiOS.ipa
```

**macOS:**
- Install via Xcode (Window → Devices and Simulators)
- Or use `xtool install HandoffiOS.ipa`

## Step 4: Trust the Developer Certificate

1. On iPhone, open **Settings**
2. Go to **General** → **VPN & Device Management**
3. Find your developer certificate (your Apple ID or team name)
4. Tap **Trust**

## Step 5: Launch the App

1. Open **Handoff** app on iPhone
2. Grant permissions when prompted:
   - Camera (for QR scanning and webcam)
   - Bluetooth (for proximity detection)
   - Local Network (for device discovery)
   - Face ID (for Linux unlock authentication)

## Step 6: Pair with Linux

**On Linux:**
```bash
# Make sure daemon is running
systemctl --user start handoffd

# Generate QR code
handoff pair
```

**On iPhone:**
1. Tap the **Devices** tab
2. Tap the QR scanner icon (top right)
3. Scan the QR code displayed on Linux
4. Verify the fingerprint matches
5. Tap **Pair**

## Step 7: Test the Connection

**On Linux:**
```bash
# Check status
handoff status

# Test clipboard sync
echo "Hello from Linux" | handoff clipboard set
# Check iPhone clipboard - should show "Hello from Linux"

# Test notification
handoff notify "Test from Linux"
# Should show notification on iPhone
```

**On iPhone:**
- Copy text in any app
- Open Handoff app → Clipboard tab
- Text should appear in clipboard history
- Tap to sync to Linux

## Troubleshooting

**App won't install:**
- Make sure Developer Mode is enabled
- Check that you're using the correct Apple ID/team
- Try restarting both devices

**Can't find Linux device:**
- Ensure both devices are on the same WiFi network
- Check that `handoffd` is running: `systemctl --user status handoffd`
- Verify firewall allows port 9877

**Pairing fails:**
- Regenerate QR code: `handoff pair`
- Forget device in Handoff app and try again
- Check logs: `journalctl --user -u handoffd -f`

**Background sync not working:**
- iOS limits background execution
- App must be opened periodically to sync
- Background fetch runs every 15+ minutes (system decides)

## Next Steps

- **Linux GUI:** Run `handoff-gui` for desktop app
- **Hyprland widgets:** `quickshell --config ~/Projects/handoff/ui/shell.conf`
- **Camera as webcam:** Enable in config, load `v4l2loopback` module
- **Proximity lock:** Configure RSSI thresholds in `~/.config/handoff/config.toml`
