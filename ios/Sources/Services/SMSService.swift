import Foundation

class SMSService: ObservableObject {
    @Published var messages: [SMSMessage] = []

    // ponytail: iOS has no SMS API — this relies on notification forwarding
    // from the iOS notification stream. Will need user to grant notification access.

    func forwardToDaemon(_ message: SMSMessage) {
        // TODO: send via ConnectionManager
    }
}

struct SMSMessage: Identifiable, Codable {
    let id: String
    let sender: String
    let body: String
    let timestamp: Date
}
