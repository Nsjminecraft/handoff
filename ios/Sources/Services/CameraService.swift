import Foundation
import AVFoundation

class CameraService: ObservableObject {
    @Published var isStreaming = false

    private let session = AVCaptureSession()

    func startSession() {
        guard let device = AVCaptureDevice.default(.builtInWideAngleCamera, for: .video, position: .front) else { return }
        guard let input = try? AVCaptureDeviceInput(device: device) else { return }

        session.beginConfiguration()
        if session.canAddInput(input) { session.addInput(input) }
        session.commitConfiguration()
        session.startRunning()
        isStreaming = true
    }

    func stopSession() {
        session.stopRunning()
        isStreaming = false
    }
}
