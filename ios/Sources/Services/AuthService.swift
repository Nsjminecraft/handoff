import Foundation
import LocalAuthentication

class AuthService: ObservableObject {
    @Published var isAuthenticated = false

    func authenticateWithFaceID() async throws -> Data {
        let context = LAContext()
        var error: NSError?

        guard context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error) else {
            throw AuthError.biometricsUnavailable
        }

        _ = try await context.evaluatePolicy(
            .deviceOwnerAuthenticationWithBiometrics,
            localizedReason: "Authenticate to unlock your Linux machine"
        )

        isAuthenticated = true
        // TODO: sign a time-limited token with device key
        return Data()
    }

    enum AuthError: Error {
        case biometricsUnavailable
        case authenticationFailed
    }
}
