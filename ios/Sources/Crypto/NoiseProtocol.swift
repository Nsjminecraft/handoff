import Foundation
import CryptoKit

/// Noise_XX_25519_ChaChaPoly_SHA256 implementation using CryptoKit.
/// Matches the `snow` crate on the Rust daemon side.
class NoiseSession {
    private let isInitiator: Bool
    private let localStatic: Curve25519.KeyAgreement.PrivateKey
    private var remoteStaticPub: Curve25519.KeyAgreement.PublicKey?
    private var localEphemeral: Curve25519.KeyAgreement.PrivateKey?
    private var remoteEphemeralPub: Curve25519.KeyAgreement.PublicKey?

    private var ck: Data   // chaining key
    private var h: Data    // handshake hash
    private var k: Data?   // current encryption key
    private var nonce: UInt64 = 0

    // Transport mode keys
    private var sendKey: Data?
    private var recvKey: Data?
    private var sendNonce: UInt64 = 0
    private var recvNonce: UInt64 = 0

    private var transportMode = false
    private var messageIndex = 0

    static let protocolName = "Noise_XX_25519_ChaChaPoly_SHA256"

    init(isInitiator: Bool, localStaticKey: Curve25519.KeyAgreement.PrivateKey) {
        self.isInitiator = isInitiator
        self.localStatic = localStaticKey

        // Initialize symmetric state: h = protocol_name padded to 32 bytes
        let nameData = Data(Self.protocolName.utf8)
        var padded = nameData
        if padded.count < 32 { padded.append(Data(count: 32 - padded.count)) }
        self.h = padded
        self.ck = padded
        // Empty prologue: MixHash(empty)
        self.h = NoiseSession.sha256(self.h)
    }

    // MARK: - Handshake

    /// Write the next handshake message to send.
    func writeHandshakeMessage(payload: Data = Data()) throws -> Data {
        var out = Data()

        if !transportMode {
            let msgIndex = handshakeMessageIndex()

            if msgIndex == 0 {
                // -> e
                let eph = try Curve25519.KeyAgreement.PrivateKey()
                localEphemeral = eph
                let ephPub = eph.publicKey.rawRepresentation
                out.append(ephPub)
                mixHash(ephPub)

            } else if msgIndex == 1 {
                // <- e, ee, s, es  (responder writes)
                let eph = try Curve25519.KeyAgreement.PrivateKey()
                localEphemeral = eph
                let ephPub = eph.publicKey.rawRepresentation
                out.append(ephPub)
                mixHash(ephPub)

                // ee
                guard let remoteEph = remoteEphemeralPub else { throw NoiseError.missingRemoteKey }
                let dhEE = try dh(localEphemeral!, remoteEph)
                mixKey(dhEE)

                // s (encrypted)
                let sPub = localStatic.publicKey.rawRepresentation
                out.append(encryptAndHash(sPub))

                // es
                guard let remoteEph2 = remoteEphemeralPub else { throw NoiseError.missingRemoteKey }
                let dhES = try dh(localStatic, remoteEph2)
                mixKey(dhES)

                // payload (encrypted)
                out.append(encryptAndHash(payload))

            } else if msgIndex == 2 {
                // -> s, se  (initiator writes)
                let sPub = localStatic.publicKey.rawRepresentation
                out.append(encryptAndHash(sPub))

                // se
                guard let remoteEph = remoteEphemeralPub else { throw NoiseError.missingRemoteKey }
                let dhSE = try dh(localEphemeral!, remoteEph)
                mixKey(dhSE)

                // payload (encrypted)
                out.append(encryptAndHash(payload))
            }

            advanceMessageIndex()
        }

        return out
    }

    /// Read a handshake message received from the peer.
    func readHandshakeMessage(_ data: Data) throws -> Data {
        var remaining = data

        if !transportMode {
            let msgIndex = handshakeMessageIndex()

            if msgIndex == 0 {
                // -> e  (initiator sent ephemeral)
                guard remaining.count >= 32 else { throw NoiseError.invalidMessage }
                let ephPubBytes = remaining.prefix(32)
                remaining = remaining.dropFirst(32)
                remoteEphemeralPub = try Curve25519.KeyAgreement.PublicKey(rawRepresentation: ephPubBytes)
                mixHash(ephPubBytes)

            } else if msgIndex == 1 {
                // <- e, ee, s, es  (responder sent)
                guard remaining.count >= 32 else { throw NoiseError.invalidMessage }
                let ephPubBytes = remaining.prefix(32)
                remaining = remaining.dropFirst(32)
                remoteEphemeralPub = try Curve25519.KeyAgreement.PublicKey(rawRepresentation: ephPubBytes)
                mixHash(ephPubBytes)

                // ee
                guard let localEph = localEphemeral else { throw NoiseError.missingLocalEphemeral }
                let dhEE = try dh(localEph, remoteEphemeralPub!)
                mixKey(dhEE)

                // s (encrypted) — decrypt 48 bytes (32 key + 16 tag)
                let encryptedS = remaining.prefix(48)
                remaining = remaining.dropFirst(48)
                let sPubBytes = try decryptAndHash(encryptedS)
                remoteStaticPub = try Curve25519.KeyAgreement.PublicKey(rawRepresentation: sPubBytes)

                // es
                let dhES = try dh(localEph, remoteStaticPub!)
                mixKey(dhES)

                // payload (encrypted) — rest is encrypted payload
                let payloadBytes = try decryptAndHash(remaining)
                remaining = Data(payloadBytes)

            } else if msgIndex == 2 {
                // -> s, se  (initiator sent)
                // s (encrypted)
                let encryptedS = remaining.prefix(48)
                remaining = remaining.dropFirst(48)
                let sPubBytes = try decryptAndHash(encryptedS)
                remoteStaticPub = try Curve25519.KeyAgreement.PublicKey(rawRepresentation: sPubBytes)

                // se
                guard let localEph = localEphemeral else { throw NoiseError.missingLocalEphemeral }
                let dhSE = try dh(localEph, remoteStaticPub!)
                mixKey(dhSE)

                // payload (encrypted)
                let payloadBytes = try decryptAndHash(remaining)
                remaining = Data(payloadBytes)
            }

            advanceMessageIndex()
        }

        return remaining
    }

    var isHandshakeFinished: Bool { transportMode }

    var remoteStaticPublicKey: Curve25519.KeyAgreement.PublicKey? { remoteStaticPub }

    // MARK: - Transport

    func encrypt(_ plaintext: Data) throws -> Data {
        guard let key = sendKey else { throw NoiseError.notInTransportMode }
        let nonceData = makeNonce(sendNonce)
        let symmetricKey = SymmetricKey(data: key)
        let sealed = try ChaChaPoly.seal(plaintext, using: symmetricKey, nonce: ChaChaPoly.Nonce(data: nonceData), authenticating: Data())
        sendNonce += 1
        return sealed.ciphertext + sealed.tag
    }

    func decrypt(_ ciphertext: Data) throws -> Data {
        guard let key = recvKey else { throw NoiseError.notInTransportMode }
        guard ciphertext.count >= 16 else { throw NoiseError.invalidMessage }
        let ct = ciphertext.dropLast(16)
        let tag = ciphertext.suffix(16)
        let nonceData = makeNonce(recvNonce)
        let symmetricKey = SymmetricKey(data: key)
        let sealedBox = try ChaChaPoly.SealedBox(nonce: ChaChaPoly.Nonce(data: nonceData), ciphertext: Data(ct), tag: Data(tag))
        recvNonce += 1
        return try ChaChaPoly.open(sealedBox, using: symmetricKey, authenticating: Data())
    }

    // MARK: - Internal

    private func handshakeMessageIndex() -> Int {
        return messageIndex
    }

    private func advanceMessageIndex() {
        messageIndex += 1
        if messageIndex >= 3 {
            transportMode = true
            try? split()
        }
    }

    private func dh(_ priv: Curve25519.KeyAgreement.PrivateKey, _ pub: Curve25519.KeyAgreement.PublicKey) throws -> Data {
        let shared = try priv.sharedSecretFromKeyAgreement(with: pub)
        return shared.withUnsafeBytes { Data($0) }
    }

    private func mixHash(_ data: Data) {
        h = NoiseSession.sha256(h + data)
    }

    private func mixKey(_ ikm: Data) {
        let (newCk, newK) = NoiseSession.hkdf(ck: ck, ikm: ikm)
        ck = newCk
        k = newK
        nonce = 0
    }

    private func encryptAndHash(_ plaintext: Data) -> Data {
        guard let keyData = k else {
            // No key yet — plaintext passes through, just mix hash
            mixHash(plaintext)
            return plaintext
        }
        let nonceData = makeNonce(nonce)
        let symmetricKey = SymmetricKey(data: keyData)
        let sealed = try! ChaChaPoly.seal(plaintext, using: symmetricKey, nonce: ChaChaPoly.Nonce(data: nonceData), authenticating: h)
        nonce += 1
        let ciphertext = sealed.ciphertext + sealed.tag
        mixHash(ciphertext)
        return ciphertext
    }

    private func decryptAndHash(_ ciphertext: Data) throws -> Data {
        guard let keyData = k else {
            mixHash(ciphertext)
            return ciphertext
        }
        guard ciphertext.count >= 16 else { throw NoiseError.invalidMessage }
        let nonceData = makeNonce(nonce)
        let symmetricKey = SymmetricKey(data: keyData)
        let ct = ciphertext.dropLast(16)
        let tag = ciphertext.suffix(16)
        let sealedBox = try ChaChaPoly.SealedBox(nonce: ChaChaPoly.Nonce(data: nonceData), ciphertext: Data(ct), tag: Data(tag))
        nonce += 1
        let plaintext = try ChaChaPoly.open(sealedBox, using: symmetricKey, authenticating: h)
        mixHash(ciphertext)
        return plaintext
    }

    private func split() throws {
        let (k1, k2) = NoiseSession.hkdf(ck: ck, ikm: Data())
        if isInitiator {
            sendKey = k1
            recvKey = k2
        } else {
            sendKey = k2
            recvKey = k1
        }
        sendNonce = 0
        recvNonce = 0
    }

    private func makeNonce(_ n: UInt64) -> Data {
        var data = Data(count: 4) // 4 zero bytes
        var counter = n
        data.append(Data(bytes: &counter, count: 8))
        return data
    }

    // MARK: - Crypto helpers

    private static func sha256(_ data: Data) -> Data {
        let digest = SHA256.hash(data: data)
        return Data(digest)
    }

    private static func hkdf(ck: Data, ikm: Data) -> (Data, Data) {
        // Extract
        let tempKey = hmacSHA256(key: ck, data: ikm)
        // Expand
        let out1 = hmacSHA256(key: tempKey, data: Data([0x01]))
        let out2 = hmacSHA256(key: tempKey, data: out1 + Data([0x02]))
        return (out1, out2)
    }

    private static func hmacSHA256(key: Data, data: Data) -> Data {
        let symKey = SymmetricKey(data: key)
        let mac = HMAC<SHA256>.authenticationCode(for: data, using: symKey)
        return Data(mac)
    }

    enum NoiseError: Error {
        case missingRemoteKey
        case missingLocalEphemeral
        case invalidMessage
        case notInTransportMode
    }
}
