import Foundation

public enum ControlAPIError: Error, Equatable {
    case invalidResponse
    case http(status: Int, body: String)
}

public struct ControlAPI: Sendable {
    private let baseURL: URL
    private let token: String
    private let session: URLSession
    private let decoder: JSONDecoder
    private let encoder: JSONEncoder

    public init(
        baseURL: URL,
        token: String,
        session: URLSession = .shared
    ) {
        self.baseURL = baseURL
        self.token = token
        self.session = session
        self.decoder = JSONDecoder()
        self.encoder = JSONEncoder()
    }

    public func trip() async throws -> DriverTrip {
        try await request(path: "/api/trip/(token)", method: "GET")
    }

    public func complete(pointId: String) async throws -> DriverTrip {
        try await request(
            path: "/api/trip/(token)/complete/(pointId)",
            method: "POST"
        )
    }

    public func reorder(
        basedOnRevision: UInt64,
        openPointIds: [String]
    ) async throws -> DriverTrip {
        let body = try encoder.encode(
            RouteOrderRequest(
                basedOnRevision: basedOnRevision,
                openPointIds: openPointIds
            )
        )
        return try await request(
            path: "/api/trip/(token)/route/order",
            method: "POST",
            body: body
        )
    }

    private func request<T: Decodable>(
        path: String,
        method: String,
        body: Data? = nil
    ) async throws -> T {
        let url = baseURL.appending(path: path)
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.cachePolicy = .reloadIgnoringLocalAndRemoteCacheData
        if let body {
            request.httpBody = body
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }

        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw ControlAPIError.invalidResponse
        }
        guard (200...299).contains(http.statusCode) else {
            throw ControlAPIError.http(
                status: http.statusCode,
                body: String(data: data, encoding: .utf8) ?? ""
            )
        }
        return try decoder.decode(T.self, from: data)
    }
}
