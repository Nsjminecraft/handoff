import SwiftUI
import AVFoundation
import CryptoKit

struct PairingView: View {
    @EnvironmentObject var connectionManager: ConnectionManager
    @State private var scannedDevice: ScannedDevice?
    @State private var isPairing = false
    @State private var pairingComplete = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(spacing: 20) {
            if pairingComplete {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 80))
                    .foregroundStyle(.green)
                Text("Pairing Complete")
                    .font(.title2)
            } else if let device = scannedDevice {
                // Show scanned device info for verification
                VStack(spacing: 12) {
                    Image(systemName: "desktopcomputer")
                        .font(.system(size: 60))
                        .foregroundStyle(.blue)

                    Text(device.deviceName)
                        .font(.title2)
                        .fontWeight(.semibold)

                    Text("Address: \(device.addr):\(device.port)")
                        .font(.caption)
                        .foregroundStyle(.secondary)

                    VStack(spacing: 4) {
                        Text("Public Key Fingerprint")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        Text(device.fingerprint)
                            .font(.system(.body, design: .monospaced))
                            .fontWeight(.medium)
                    }
                    .padding(.top, 8)

                    if isPairing {
                        ProgressView("Establishing trust...")
                            .padding(.top)
                    } else {
                        Button("Confirm & Pair") {
                            Task { await pair(with: device) }
                        }
                        .buttonStyle(.borderedProminent)
                        .padding(.top)

                        Button("Cancel") {
                            scannedDevice = nil
                        }
                        .foregroundStyle(.secondary)
                    }
                }
            } else {
                // QR Scanner
                QRScannerView { result in
                    handleScan(result)
                }
                .frame(maxWidth: 300, maxHeight: 300)
                .clipShape(RoundedRectangle(cornerRadius: 16))

                if let error = errorMessage {
                    Text(error)
                        .font(.caption)
                        .foregroundStyle(.red)
                }

                Text("Scan the QR code displayed on your Linux machine")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
        }
        .padding()
        .navigationTitle("Pair Device")
    }

    private func handleScan(_ result: Result<String, QRScannerError>) {
        switch result {
        case .success(let payload):
            do {
                let data = payload.data(using: .utf8)!
                let json = try JSONDecoder().decode([String: String].self, from: data)
                guard let deviceId = json["device_id"],
                      let deviceName = json["device_name"],
                      let wsPort = json["ws_port"],
                      let pubkeyB64 = json["noise_pubkey"],
                      let port = UInt16(wsPort),
                      let pubkeyData = Data(base64Encoded: pubkeyB64) else {
                    errorMessage = "Invalid QR code format"
                    return
                }

                let fingerprint = Self.computeFingerprint(pubkeyData)
                scannedDevice = ScannedDevice(
                    deviceId: deviceId,
                    deviceName: deviceName,
                    addr: "", // Will be resolved from mDNS or WS connect
                    port: port,
                    publicKeyData: pubkeyData,
                    fingerprint: fingerprint
                )
            } catch {
                errorMessage = "Failed to parse QR code: \(error.localizedDescription)"
            }
        case .failure:
            errorMessage = "Scan failed. Try again."
        }
    }

    private func pair(with device: ScannedDevice) async {
        isPairing = true
        do {
            try await connectionManager.pairWithDaemon(
                deviceId: device.deviceId,
                deviceName: device.deviceName,
                addr: device.addr,
                port: device.port,
                daemonPubKey: device.publicKeyData
            )
            pairingComplete = true
        } catch {
            errorMessage = "Pairing failed: \(error.localizedDescription)"
            isPairing = false
        }
    }

    static func computeFingerprint(_ pubkeyData: Data) -> String {
        let hash = SHA256.hash(data: pubkeyData)
        let hashData = Data(hash)
        return hashData.prefix(4).map { String(format: "%02x", $0) }.joined(separator: " ")
    }
}

struct ScannedDevice: Identifiable {
    let id = UUID()
    let deviceId: String
    let deviceName: String
    let addr: String
    let port: UInt16
    let publicKeyData: Data
    let fingerprint: String
}

// MARK: - QR Scanner (AVFoundation)

struct QRScannerView: UIViewRepresentable {
    let onScan: (Result<String, QRScannerError>) -> Void

    func makeUIView(context: Context) -> QRScannerUIView {
        let view = QRScannerUIView()
        view.onScan = onScan
        return view
    }

    func updateUIView(_ uiView: QRScannerUIView, context: Context) {}
}

enum QRScannerError: Error {
    case scanFailed
}

class QRScannerUIView: UIView, @preconcurrency AVCaptureMetadataOutputObjectsDelegate {
    var onScan: ((Result<String, QRScannerError>) -> Void)?

    private let captureSession = AVCaptureSession()
    private var previewLayer: AVCaptureVideoPreviewLayer?
    private var hasScanned = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        setupCamera()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setupCamera()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        previewLayer?.frame = bounds
    }

    private func setupCamera() {
        AVCaptureDevice.requestAccess(for: .video) { granted in
            guard granted else { return }
            self.startCaptureSession()
        }
    }

    private func startCaptureSession() {
        guard let videoCaptureDevice = AVCaptureDevice.default(for: .video) else { return }

        guard let videoInput = try? AVCaptureDeviceInput(device: videoCaptureDevice) else { return }

        guard captureSession.canAddInput(videoInput) else { return }
        captureSession.addInput(videoInput)

        let metadataOutput = AVCaptureMetadataOutput()

        guard captureSession.canAddOutput(metadataOutput) else { return }
        captureSession.addOutput(metadataOutput)

        metadataOutput.setMetadataObjectsDelegate(self, queue: DispatchQueue.main)
        metadataOutput.metadataObjectTypes = [.qr]

        let preview = AVCaptureVideoPreviewLayer(session: captureSession)
        preview.videoGravity = .resizeAspectFill
        layer.addSublayer(preview)
        previewLayer = preview

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            self?.captureSession.startRunning()
        }
    }

    nonisolated func metadataOutput(_ output: AVCaptureMetadataOutput,
                        didOutput metadataObjects: [AVMetadataObject],
                        from connection: AVCaptureConnection) {
        guard !hasScanned else { return }

        if let metadataObject = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
           metadataObject.type == .qr,
           let stringValue = metadataObject.stringValue {
            hasScanned = true
            captureSession.stopRunning()
            Task { @MainActor in
                onScan?(.success(stringValue))
            }
        }
    }
}
