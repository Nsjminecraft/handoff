import SwiftUI

struct CameraView: View {
    @StateObject private var camera = CameraService()

    var body: some View {
        VStack {
            ZStack {
                Rectangle()
                    .fill(.black)
                    .aspectRatio(16/9, contentMode: .fit)

                if camera.isStreaming {
                    // TODO: render frames from CameraService
                    Text("Streaming...").foregroundStyle(.white)
                } else {
                    Image(systemName: "video.slash")
                        .font(.largeTitle)
                        .foregroundStyle(.gray)
                }
            }

            Button(camera.isStreaming ? "Stop" : "Start Camera") {
                if camera.isStreaming {
                    camera.stopSession()
                } else {
                    camera.startSession()
                }
            }
            .buttonStyle(.borderedProminent)
        }
        .padding()
        .navigationTitle("Camera")
    }
}
