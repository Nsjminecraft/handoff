import Foundation
import UIKit
import Combine

class ClipboardService: ObservableObject {
    @Published var lastItem: ClipboardItem?

    private var pollTimer: Timer?
    private var changeCount: Int = 0

    func startMonitoring() {
        changeCount = UIPasteboard.general.changeCount
        pollTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            self?.checkForChanges()
        }
    }

    func stopMonitoring() {
        pollTimer?.invalidate()
        pollTimer = nil
    }

    private func checkForChanges() {
        let pb = UIPasteboard.general
        guard pb.changeCount != changeCount else { return }
        changeCount = pb.changeCount

        if let string = pb.string {
            lastItem = ClipboardItem(type: .text, data: Data(string.utf8))
        } else if let image = pb.image {
            lastItem = ClipboardItem(type: .image, data: image.pngData() ?? Data())
        }
    }
}

struct ClipboardItem: Identifiable {
    enum ItemType { case text, image, file }
    let id = UUID()
    let type: ItemType
    let data: Data
}
