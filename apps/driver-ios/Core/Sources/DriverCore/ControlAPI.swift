import Foundation

public enum ControlAPIError: Error, Equatable {
    case invalidResponse
    case http(status: Int, body: String)
}

public actor ControlAPI {
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
        try await requestJSON(path: "/api/trip/\(token)", method: "GET")
    }

    public func complete(pointId: String) async throws -> DriverTrip {
        try await requestJSON(
            path: "/api/trip/\(token)/complete/\(pointId)",
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
        return try await requestJSON(
            path: "/api/trip/\(token)/route/order",
            method: "POST",
            body: body,
            headers: ["Content-Type": "application/json"]
        )
    }

    public func sendLocation(_ update: LocationUpdate) async throws -> DriverLocation {
        try await requestJSON(
            path: "/api/trip/\(token)/location",
            method: "POST",
            body: try encoder.encode(update),
            headers: ["Content-Type": "application/json"]
        )
    }

    public func downloadDocument(documentId: String) async throws -> Data {
        try await requestData(
            path: "/api/trip/\(token)/documents/\(documentId)",
            method: "GET",
            accept: "*/*"
        )
    }

    public func uploadDocument(
        documentId: String = Self.newDocumentId(),
        originalName: String,
        mimeType: String,
        data: Data,
        pointId: String? = nil,
        createdAtUnixMs: UInt64 = UInt64(Date().timeIntervalSince1970 * 1000)
    ) async throws -> DriverTrip {
        var headers = [
            "Content-Type": mimeType.isEmpty ? "application/octet-stream" : mimeType,
            "X-File-Name": Self.percentEncodeHeader(originalName),
            "X-Created-At-Unix-Ms": String(createdAtUnixMs),
        ]
        if let pointId {
            headers["X-Point-Id"] = pointId
        }
        return try await requestJSON(
            path: "/api/trip/\(token)/documents/\(documentId)",
            method: "POST",
            body: data,
            headers: headers
        )
    }

    private func requestJSON<T: Decodable>(
        path: String,
        method: String,
        body: Data? = nil,
        headers: [String: String] = [:]
    ) async throws -> T {
        let data = try await requestData(
            path: path,
            method: method,
            body: body,
            headers: headers,
            accept: "application/json"
        )
        return try decoder.decode(T.self, from: data)
    }

    private func requestData(
        path: String,
        method: String,
        body: Data? = nil,
        headers: [String: String] = [:],
        accept: String
    ) async throws -> Data {
        let url = baseURL.appending(path: path)
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue(accept, forHTTPHeaderField: "Accept")
        request.cachePolicy = .reloadIgnoringLocalAndRemoteCacheData
        request.httpBody = body
        for (name, value) in headers {
            request.setValue(value, forHTTPHeaderField: name)
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
        return data
    }

    public static func newDocumentId() -> String {
        UUID().uuidString.replacingOccurrences(of: "-", with: "").lowercased()
    }

    public static func percentEncodeHeader(_ value: String) -> String {
        let allowed = CharacterSet(charactersIn: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~")
        return value.addingPercentEncoding(withAllowedCharacters: allowed) ?? value
    }
}
