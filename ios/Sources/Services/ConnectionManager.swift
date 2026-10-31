import Foundation
import Combine
import CryptoKit
import UIKit
import BackgroundTasks

@MainActor
class ConnectionManager: ObservableObject {
    @Published var devices: [Device] = []
    @Published var connectedPeers: [String] = []
    @Published var connectionState: ConnectionState = .disconnected

    private var webSocketTask: URLSessionWebSocketTask?
    private var noiseSession: NoiseSession?
    private var pingTask: Task<Void, Never>?
    private let localStaticKey: Curve25519.KeyAgreement.PrivateKey
    private let deviceId: String
    private let deviceName: String

    enum ConnectionState {
        case disconnected
        case connecting
        case connected
    }

    init() {
        // Load or generate persistent local static key
        if let savedKeyData = UserDefaults.standard.data(forKey: "noiseStaticKey"),
           let key = try? Curve25519.KeyAgreement.PrivateKey(rawRepresentation: savedKeyData) {
            localStaticKey = key
        } else {
            localStaticKey = try! Curve25519.KeyAgreement.PrivateKey()
            UserDefaults.standard.set(localStaticKey.rawRepresentation, forKey: "noiseStaticKey")
        }

        // Device identity
        if let savedId = UserDefaults.standard.string(forKey: "deviceId") {
            deviceId = savedId
        } else {
            deviceId = UUID().uuidString
            UserDefaults.standard.set(deviceId, forKey: "deviceId")
        }
        deviceName = UIDevice.current.name

        registerBackgroundTasks()
    }

    func connect(to device: Device) async {
        connectionState = .connecting
        let url = URL(string: "ws://\(device.addr):\(device.port)")!
        webSocketTask = URLSession.shared.webSocketTask(with: url)
        webSocketTask?.resume()

        do {
            // Perform Noise XX handshake (initiator)
            noiseSession = NoiseSession(isInitiator: true, localStaticKey: localStaticKey)

            // Msg 1: -> e
            let msg1 = try noiseSession!.writeHandshakeMessage()
            try await sendBinary(msg1)

            // Msg 2: <- e, ee, s, es
            let msg2Data = try await receiveBinary()
            _ = try noiseSession!.readHandshakeMessage(msg2Data)

            // Msg 3: -> s, se
            let payload = Data() // No payload for now
            let msg3 = try noiseSession!.writeHandshakeMessage(payload: payload)
            try await sendBinary(msg3)

            // Verify remote static key matches stored daemon key
            if let remotePubKey = noiseSession?.remoteStaticPublicKey,
               let storedDaemonKeyData = UserDefaults.standard.data(forKey: "daemonNoisePubKey"),
               let storedKey = try? Curve25519.KeyAgreement.PublicKey(rawRepresentation: storedDaemonKeyData) {
                // Trust binding: verify remote key matches stored key
                if remotePubKey.rawRepresentation != storedKey.rawRepresentation {
                    print("WARNING: Remote static key mismatch! Potential MITM attack.")
                    throw ConnectionError.trustMismatch
                }
            }

            // Handshake complete, send device.hello
            let helloMsg = [
                "type": "device.hello",
                "device_id": deviceId,
                "device_name": deviceName
            ] as [String : Any]
            let helloData = try JSONSerialization.data(withJSONObject: helloMsg)
            let encrypted = try noiseSession!.encrypt(helloData)
            try await sendBinary(encrypted)

            connectionState = .connected
            connectedPeers.append(device.id)
            startPingKeepalive()

            // Start receive loop
            Task { await receiveLoop() }

        } catch {
            print("Handshake failed: \(error)")
            connectionState = .disconnected
            disconnect(from: device.id)
        }
    }

    func disconnect(from deviceId: String) {
        pingTask?.cancel()
        pingTask = nil
        webSocketTask?.cancel(with: .goingAway, reason: nil)
        webSocketTask = nil
        noiseSession = nil
        connectionState = .disconnected
        connectedPeers.removeAll { $0 == deviceId }
    }

    func send(_ json: [String: Any]) async throws {
        guard let session = noiseSession else { throw NSError(domain: "ConnectionManager", code: -1) }
        let data = try JSONSerialization.data(withJSONObject: json)
        let encrypted = try session.encrypt(data)
        try await sendBinary(encrypted)
    }

    func sendClipboard(_ text: String) async throws {
        let msg = [
            "type": "clipboard.sync",
            "id": UUID().uuidString,
            "timestamp": Int(Date().timeIntervalSince1970),
            "payload": [
                "mime": "text/plain",
                "data": text
            ]
        ] as [String : Any]
        try await send(msg)
    }

    func sendNotification(title: String, body: String) async throws {
        let msg = [
            "type": "notification",
            "id": UUID().uuidString,
            "timestamp": Int(Date().timeIntervalSince1970),
            "payload": [
                "title": title,
                "body": body
            ]
        ] as [String : Any]
        try await send(msg)
    }

    // MARK: - Pairing

    func pairWithDaemon(deviceId: String, deviceName: String, addr: String, port: UInt16, daemonPubKey: Data) async throws {
        // Store daemon's public key for trust binding
        UserDefaults.standard.set(daemonPubKey, forKey: "daemonNoisePubKey")
        UserDefaults.standard.set("\(addr):\(port)", forKey: "daemonAddress")
        UserDefaults.standard.set(deviceId, forKey: "daemonDeviceId")
        UserDefaults.standard.set(deviceName, forKey: "daemonDeviceName")

        // Connect and perform pairing handshake
        let url = URL(string: "ws://\(addr):\(port)")!
        webSocketTask = URLSession.shared.webSocketTask(with: url)
        webSocketTask?.resume()

        // Perform Noise XX handshake
        noiseSession = NoiseSession(isInitiator: true, localStaticKey: localStaticKey)

        // Msg 1: -> e
        let msg1 = try noiseSession!.writeHandshakeMessage()
        try await sendBinary(msg1)

        // Msg 2: <- e, ee, s, es
        let msg2Data = try await receiveBinary()
        _ = try noiseSession!.readHandshakeMessage(msg2Data)

        // Verify remote key matches scanned QR key
        guard let remotePubKey = noiseSession?.remoteStaticPublicKey else {
            throw ConnectionError.noRemoteKey
        }
        if remotePubKey.rawRepresentation != daemonPubKey {
            throw ConnectionError.trustMismatch
        }

        // Msg 3: -> s, se with pairing payload
        let pairPayload = [
            "action": "pair",
            "ios_device_id": self.deviceId,
            "ios_device_name": self.deviceName
        ] as [String: String]
        let pairData = try JSONSerialization.data(withJSONObject: pairPayload)
        let msg3 = try noiseSession!.writeHandshakeMessage(payload: pairData)
        try await sendBinary(msg3)

        // Handshake complete, wait for pairing confirmation
        let response = try await receiveBinary()
        let decrypted = try noiseSession!.decrypt(response)
        let json = try JSONSerialization.jsonObject(with: decrypted) as? [String: Any]

        if json?["pairing_confirmed"] as? Bool == true {
            connectionState = .connected
            connectedPeers.append(deviceId)
            startPingKeepalive()
            Task { await receiveLoop() }
        } else {
            throw ConnectionError.pairingRejected
        }
    }

    // MARK: - Background

    private func registerBackgroundTasks() {
        BGTaskScheduler.shared.register(forTaskWithIdentifier: "com.handoff.refresh", using: nil) { task in
            self.handleAppRefresh(task as! BGAppRefreshTask)
        }
    }

    private func handleAppRefresh(_ task: BGAppRefreshTask) {
        scheduleAppRefresh()

        let refreshTask = Task {
            // Check clipboard for changes and send to daemon
            if connectionState == .connected, let text = UIPasteboard.general.string {
                try? await sendClipboard(text)
            }
        }

        task.expirationHandler = {
            refreshTask.cancel()
        }

        Task {
            _ = await refreshTask.result
            task.setTaskCompleted(success: true)
        }
    }

    private func scheduleAppRefresh() {
        let request = BGAppRefreshTaskRequest(identifier: "com.handoff.refresh")
        request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60) // 15 min
        try? BGTaskScheduler.shared.submit(request)
    }

    private func startPingKeepalive() {
        pingTask?.cancel()
        pingTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(30))
                guard let self = self, self.connectionState == .connected else { break }
                let ping = ["type": "ping"] as [String: Any]
                if let data = try? JSONSerialization.data(withJSONObject: ping),
                   let encrypted = try? self.noiseSession?.encrypt(data) {
                    try? await self.sendBinary(encrypted)
                }
            }
        }
    }

    // MARK: - WebSocket helpers

    private func sendBinary(_ data: Data) async throws {
        try await webSocketTask?.send(.data(data))
    }

    private func receiveBinary() async throws -> Data {
        let message = try await webSocketTask?.receive()
        guard case .data(let data) = message else {
            throw NSError(domain: "ConnectionManager", code: -2)
        }
        return data
    }

    private func receiveLoop() async {
        guard let session = noiseSession else { return }
        do {
            while true {
                let encrypted = try await receiveBinary()
                let decrypted = try session.decrypt(encrypted)
                let json = try JSONSerialization.jsonObject(with: decrypted) as? [String: Any]
                await handleMessage(json ?? [:])
            }
        } catch {
            print("Receive loop error: \(error)")
            connectionState = .disconnected
        }
    }

    private func handleMessage(_ json: [String: Any]) async {
        guard let type = json["type"] as? String else { return }

        switch type {
        case "ping":
            let pong = ["type": "pong"]
            if let data = try? JSONSerialization.data(withJSONObject: pong),
               let encrypted = try? noiseSession?.encrypt(data) {
                try? await sendBinary(encrypted)
            }

        case "clipboard.sync":
            if let payload = json["payload"] as? [String: Any],
               let text = payload["data"] as? String {
                UIPasteboard.general.string = text
            }

        case "notification":
            if let payload = json["payload"] as? [String: Any] {
                let title = payload["title"] as? String ?? ""
                let body = payload["body"] as? String ?? ""
                await NotificationService.shared.show(title: title, body: body)
            }

        default:
            print("Unknown message type: \(type)")
        }
    }

    enum ConnectionError: Error {
        case trustMismatch
        case noRemoteKey
        case pairingRejected
    }
}
