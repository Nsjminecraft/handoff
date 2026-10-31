import SwiftUI

struct ClipboardView: View {
    @StateObject private var clipboard = ClipboardService()

    var body: some View {
        List {
            if let item = clipboard.lastItem {
                Section("Current") {
                    switch item.type {
                    case .text:
                        if let str = String(data: item.data, encoding: .utf8) {
                            Text(str).textSelection(.enabled)
                        }
                    case .image:
                        if let uiImage = UIImage(data: item.data) {
                            Image(uiImage: uiImage).resizable().scaledToFit()
                        }
                    case .file:
                        Label("File (\(item.data.count) bytes)", systemImage: "doc")
                    }
                }
            }

            Section("History") {
                // TODO: load from daemon via IPC
                Text("No history yet").foregroundStyle(.secondary)
            }
        }
        .navigationTitle("Clipboard")
        .onAppear { clipboard.startMonitoring() }
        .onDisappear { clipboard.stopMonitoring() }
    }
}
