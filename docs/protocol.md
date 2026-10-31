# Handoff Wire Protocol

## Transport

- **LAN**: WebSocket over TCP (default port `49200`)
- **Cloud relay**: WebSocket over TLS (`wss://relay.handoff.local/ws`)
- **IPC** (daemon to local UI): WebSocket on `127.0.0.1:49200` or Unix socket

## Message Format

All messages are JSON encoded with a 4-byte big-endian length prefix on the WebSocket binary frame.

```json
{
  "type": "<message_type>",
  "id": "<uuid>",
  "timestamp": <unix_seconds>,
  "payload": { ... }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Dot-separated message type, e.g. `clipboard.sync` |
| `id` | string | Unique message UUID |
| `timestamp` | uint64 | Unix seconds since epoch |
| `payload` | object | Type-specific payload |

## Message Types

### Connection

| Type | Direction | Description |
|------|-----------|-------------|
| `auth` | client → server | Authenticate with device ID and signed token |
| `auth.ack` | server → client | Auth accepted |
| `auth.reject` | server → client | Auth rejected |

### Discovery

| Type | Direction | Description |
|------|-----------|-------------|
| `hello` | bidirectional | Announce device capabilities |
| `hello.ack` | bidirectional | Acknowledge capabilities |
| `ping` | bidirectional | Keepalive |
| `pong` | bidirectional | Keepalive response |

### Clipboard

| Type | Direction | Description |
|------|-----------|-------------|
| `clipboard.sync` | bidirectional | New clipboard content |
| `clipboard.request` | bidirectional | Request clipboard from peer |
| `clipboard.history` | server → client | Clipboard history list |

#### Payload: `clipboard.sync`
```json
{
  "mime": "text/plain",
  "data": "base64-encoded-content",
  "device_id": "sender-device-id"
}
```

### Notifications

| Type | Direction | Description |
|------|-----------|-------------|
| `notification.push` | bidirectional | Forward a notification |
| `notification.action` | client → server | User tapped notification action |
| `notification.dismiss` | bidirectional | Notification was dismissed |

### Handoff (URL/State Transfer)

| Type | Direction | Description |
|------|-----------|-------------|
| `handoff.url` | bidirectional | Open URL on target device |
| `handoff.app` | bidirectional | Open app/state on target device |

#### Payload: `handoff.url`
```json
{
  "url": "https://example.com",
  "target_device_id": "target-uuid",
  "source_device_id": "source-uuid"
}
```

### Proximity

| Type | Direction | Description |
|------|-----------|-------------|
| `proximity.update` | client → server | RSSI/distance update |
| `proximity.unlock` | ios → daemon | Face ID verified, unlock request |
| `proximity.lock` | daemon → ipc | Lock session |

### Camera

| Type | Direction | Description |
|------|-----------|-------------|
| `camera.start` | client → server | Begin streaming camera |
| `camera.stop` | client → server | Stop streaming |
| `camera.frame` | server → client | Raw video frame (binary) |

### Commands

| Type | Direction | Description |
|------|-----------|-------------|
| `command.exec` | ios → daemon | Execute shell command (requires confirmation) |
| `command.confirm` | daemon → ios | Request user confirmation |
| `command.result` | daemon → ios | Command output |

### SMS

| Type | Direction | Description |
|------|-----------|-------------|
| `sms.forward` | ios → daemon | Forward received SMS |
| `sms.send` | daemon → ios | Send SMS via iPhone |

## Encryption

All device-to-device communication uses the **Noise Protocol Framework** with the **XX handshake pattern**:

1. Both sides generate Curve25519 keypairs (ephemeral + static)
2. XX handshake provides mutual authentication
3. Subsequent messages encrypted with ChaChaPoly or AES-GCM
4. The relay server sees only encrypted payloads

### Pairing Flow

1. Daemon displays QR code containing: device ID, public key, IP/port
2. iOS scans QR → extracts device info
3. iOS initiates Noise XX handshake
4. Both sides verify static keys
5. Trust established, device info persisted to `~/.config/handoff/devices.json`
