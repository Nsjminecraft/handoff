import Foundation

struct ProtocolMessage: Codable {
    let type: String
    let id: String
    let timestamp: UInt64
    let payload: [String: AnyCodable]

    init(type: String, payload: [String: AnyCodable] = [:]) {
        self.type = type
        self.id = UUID().uuidString
        self.timestamp = UInt64(Date().timeIntervalSince1970)
        self.payload = payload
    }
}

// Simple AnyCodable wrapper for JSON serialization
struct AnyCodable: Codable {
    let value: Any

    init(_ value: Any) {
        self.value = value
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let string = try? container.decode(String.self) {
            value = string
        } else if let int = try? container.decode(Int.self) {
            value = int
        } else if let double = try? container.decode(Double.self) {
            value = double
        } else if let bool = try? container.decode(Bool.self) {
            value = bool
        } else {
            value = NSNull()
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch value {
        case let string as String: try container.encode(string)
        case let int as Int: try container.encode(int)
        case let double as Double: try container.encode(double)
        case let bool as Bool: try container.encode(bool)
        default: try container.encodeNil()
        }
    }
}
