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
    private static let tokenPattern = /^[A-Za-z0-9_-]{16,128}$/

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
        guard token.wholeMatch(of: tokenPattern) != nil else {
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
