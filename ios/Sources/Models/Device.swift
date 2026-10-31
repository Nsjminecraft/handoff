import Foundation

struct Device: Identifiable, Codable {
    let id: String
    var name: String
    var addr: String
    var port: UInt16
    var publicKey: Data?
    var lastSeen: Date
    var isOnline: Bool = false

    init(id: String = UUID().uuidString, name: String, addr: String, port: UInt16, publicKey: Data?, lastSeen: Date = Date()) {
        self.id = id
        self.name = name
        self.addr = addr
        self.port = port
        self.publicKey = publicKey
        self.lastSeen = lastSeen
    }
}
