import SwiftUI

struct DeviceListView: View {
    @EnvironmentObject var connectionManager: ConnectionManager
    @EnvironmentObject var discoveryService: DiscoveryService

    var body: some View {
        List {
            Section("Discovered Devices") {
                ForEach(discoveryService.discoveredDevices) { device in
                    HStack {
                        Image(systemName: device.isOnline ? "circle.fill" : "circle")
                            .foregroundStyle(device.isOnline ? .green : .gray)
                        VStack(alignment: .leading) {
                            Text(device.name).font(.headline)
                            Text("\(device.addr):\(device.port)")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .contentShape(Rectangle())
                    .onTapGesture {
                        Task {
                            await connectionManager.connect(to: device)
                        }
                    }
                }
            }

            Section("Connected") {
                ForEach(connectionManager.connectedPeers, id: \.self) { id in
                    Label(id, systemImage: "link")
                }
            }
        }
        .navigationTitle("Handoff")
        .toolbar {
            Button {
                discoveryService.stopBrowsing()
                discoveryService.startBrowsing()
            } label: {
                Image(systemName: "arrow.clockwise")
            }

            NavigationLink {
                PairingView()
            } label: {
                Image(systemName: "qrcode.viewfinder")
            }
        }
    }
}
