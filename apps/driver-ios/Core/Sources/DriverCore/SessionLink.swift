import Foundation

public struct SessionTarget: Equatable, Sendable {
    public let baseURL: URL
    public let token: String

    public init(baseURL: URL, token: String) {
        self.baseURL = baseURL
        self.token = token
    }
}

public enum SessionLinkError: Error, Equatable {
    case invalidURL
    case unsupportedScheme
    case invalidPath
    case invalidToken
}

public enum SessionLink {
    public static func parseDriverURL(_ raw: String) throws -> SessionTarget {
        guard let url = URL(string: raw.trimmingCharacters(in: .whitespacesAndNewlines)),
              let scheme = url.scheme?.lowercased(),
              let host = url.host else {
            throw SessionLinkError.invalidURL
        }
        guard scheme == "http" || scheme == "https" else {
            throw SessionLinkError.unsupportedScheme
        }

        let segments = url.pathComponents.filter { $0 != "/" }
        guard segments.count == 2, segments[0] == "driver" else {
            throw SessionLinkError.invalidPath
        }
        let token = segments[1]
        let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "_-"))
        guard (16...128).contains(token.count),
              token.unicodeScalars.allSatisfy({ allowed.contains($0) }) else {
            throw SessionLinkError.invalidToken
        }

        var components = URLComponents()
        components.scheme = scheme
        components.host = host
        components.port = url.port
        guard let baseURL = components.url else {
            throw SessionLinkError.invalidURL
        }
        return SessionTarget(baseURL: baseURL, token: token)
    }
}
