import Foundation
import Security

struct SecureSessionStore {
    private let defaults: UserDefaults

    private let service = "com.romanzavada.cargodispatch.driver.session"
    private let account = "driver-session-token"
    private let legacyURLKey = "cargo_dispatch_driver_url"
    private let baseURLKey = "cargo_dispatch_driver_base_url"

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    func loadDriverURL() -> String? {
        // Command-line UserDefaults and r1-r5 persisted raw URLs both arrive
        // through this key. They are accepted only in memory so a successful
        // reconnect can migrate them into Keychain.
        if let legacy = defaults.string(forKey: legacyURLKey)?
            .trimmingCharacters(in: .whitespacesAndNewlines),
           !legacy.isEmpty {
            return legacy
        }

        guard
            let base = defaults.string(forKey: baseURLKey)?
                .trimmingCharacters(in: .whitespacesAndNewlines),
            !base.isEmpty,
            let token = readToken(),
            !token.isEmpty
        else {
            return nil
        }

        return "\(base.trimmingCharacters(in: CharacterSet(charactersIn: "/")))/driver/\(token)"
    }

    func save(driverURL: String) throws {
        let target = try SessionLink.parseDriverURL(driverURL)
        let base = target.baseURL.absoluteString.trimmingCharacters(
            in: CharacterSet(charactersIn: "/")
        )

        do {
            try writeToken(target.token)
            defaults.set(base, forKey: baseURLKey)
            defaults.removeObject(forKey: legacyURLKey)
        } catch {
            clear()
            throw error
        }
    }

    func clear() {
        defaults.removeObject(forKey: legacyURLKey)
        defaults.removeObject(forKey: baseURLKey)

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
        ]
        SecItemDelete(query as CFDictionary)
    }

    private func readToken() -> String? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
            kSecReturnData as String: kCFBooleanTrue as Any,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess,
              let data = result as? Data,
              let token = String(data: data, encoding: .utf8)
        else {
            return nil
        }
        return token
    }

    private func writeToken(_ token: String) throws {
        let data = Data(token.utf8)
        let lookup: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
        ]

        let updateStatus = SecItemUpdate(
            lookup as CFDictionary,
            [kSecValueData as String: data] as CFDictionary
        )
        if updateStatus == errSecSuccess {
            return
        }
        guard updateStatus == errSecItemNotFound else {
            throw KeychainSessionError(status: updateStatus)
        }

        var add = lookup
        add[kSecValueData as String] = data
        add[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        let addStatus = SecItemAdd(add as CFDictionary, nil)
        guard addStatus == errSecSuccess else {
            throw KeychainSessionError(status: addStatus)
        }
    }
}

private struct KeychainSessionError: Error {
    let status: OSStatus
}
