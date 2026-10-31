import SwiftUI

@main
struct HandoffApp: App {
    @StateObject private var connectionManager = ConnectionManager()
    @StateObject private var discoveryService = DiscoveryService()

    var body: some Scene {
        WindowGroup {
            TabView {
                NavigationStack {
                    DeviceListView()
                }
                .tabItem {
                    Label("Devices", systemImage: "antenna.radiowaves.left.and.right")
                }

                NavigationStack {
                    ClipboardView()
                }
                .tabItem {
                    Label("Clipboard", systemImage: "doc.on.clipboard")
                }

                NavigationStack {
                    CameraView()
                }
                .tabItem {
                    Label("Camera", systemImage: "camera")
                }
            }
            .environmentObject(connectionManager)
            .environmentObject(discoveryService)
            .task {
                discoveryService.startBrowsing()
            }
        }
    }
}
