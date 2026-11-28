import CryptoKit
import Foundation

public enum ControlAPIError: Error, Equatable {
    case invalidResponse
    case http(status: Int, body: String)
}

/// How much of a file from the driver Control already holds.
public struct UploadStatus: Codable, Equatable, Sendable {
    public let received: UInt64
    public let complete: Bool
    public let sha256: String?

    public init(received: UInt64, complete: Bool, sha256: String? = nil) {
        self.received = received
        self.complete = complete
        self.sha256 = sha256
    }
}

/// What a file transfer needs from Control.
public protocol FileTransferAPI: Sendable {
    func uploadStatus(documentId: String) async throws -> UploadStatus
    func uploadPart(
        _ item: PendingMediaMetadata,
        offset: UInt64,
        total: UInt64,
        part: Data,
        sha256: String
    ) async throws -> UploadStatus
    /// The one request upload of Control 0.3.0 and earlier.
    func uploadWhole(_ item: PendingMediaMetadata, file: URL) async throws
    /// Appends the part of a document that starts at `offset` to `destination`.
    /// Returns false when Control sent the whole file instead, which replaces
    /// what `destination` held.
    func downloadPart(
        documentId: String,
        offset: UInt64,
        length: Int,
        destination: URL
    ) async throws -> Bool
}

/// Files travel in parts, so a broken connection costs one part, not the file.
/// The sender asks Control what it already holds and goes on from there.
public enum FileTransfer {
    public static let partBytes = 1024 * 1024

    public static func upload(
        api: any FileTransferAPI,
        item: PendingMediaMetadata,
        file: URL,
        partBytes: Int = FileTransfer.partBytes
    ) async throws {
        let total = try size(of: file)
        var status: UploadStatus
        do {
            status = try await api.uploadStatus(documentId: item.documentId)
        } catch let ControlAPIError.http(code, _) where code == 404 || code == 405 {
            try await api.uploadWhole(item, file: file)
            return
        }
        var misfits = 0
        while !status.complete {
            let offset = status.received
            guard offset <= total else {
                throw ControlAPIError.http(status: 409, body: "another file")
            }
            let length = Int(min(UInt64(partBytes), total - offset))
            let part = try read(file, offset: offset, length: length)
            do {
                status = try await api.uploadPart(
                    item,
                    offset: offset,
                    total: total,
                    part: part,
                    sha256: sha256Hex(part)
                )
                if !status.complete && status.received <= offset {
                    throw URLError(.badServerResponse)
                }
            } catch let ControlAPIError.http(code, _) where code == 409 && misfits == 0 {
                // Control holds something else than expected: ask once more.
                misfits += 1
                status = try await api.uploadStatus(documentId: item.documentId)
            }
        }
    }

    /// Completes `partial` with the rest of a document of `size` bytes. A file
    /// that does not match `sha256` is thrown away.
    public static func download(
        api: any FileTransferAPI,
        documentId: String,
        size expected: UInt64,
        sha256: String?,
        partial: URL,
        partBytes: Int = FileTransfer.partBytes
    ) async throws {
        let files = FileManager.default
        try files.createDirectory(
            at: partial.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        if files.fileExists(atPath: partial.path), try size(of: partial) > expected {
            try files.removeItem(at: partial)
        }
        if !files.fileExists(atPath: partial.path) {
            try Data().write(to: partial)
        }
        while try size(of: partial) < expected {
            let held = try size(of: partial)
            let length = Int(min(UInt64(partBytes), expected - held))
            let more = try await api.downloadPart(
                documentId: documentId,
                offset: held,
                length: length,
                destination: partial
            )
            if !more { break }
            if try size(of: partial) <= held { throw URLError(.badServerResponse) }
        }
        var matches = try size(of: partial) == expected
        if matches, let sha256, !sha256.isEmpty {
            matches = try sha256Hex(fileAt: partial) == sha256.lowercased()
        }
        if !matches {
            try? files.removeItem(at: partial)
            throw URLError(.cannotDecodeContentData)
        }
    }

    public static func sha256Hex(_ data: Data) -> String {
        hex(SHA256.hash(data: data))
    }

    public static func sha256Hex(fileAt url: URL) throws -> String {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        var hasher = SHA256()
        while let block = try handle.read(upToCount: 64 * 1024), !block.isEmpty {
            hasher.update(data: block)
        }
        return hex(hasher.finalize())
    }

    private static func hex(_ digest: SHA256.Digest) -> String {
        digest.map { String(format: "%02x", $0) }.joined()
    }

    private static func size(of url: URL) throws -> UInt64 {
        let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
        return (attributes[.size] as? NSNumber)?.uint64Value ?? 0
    }

    private static func read(_ url: URL, offset: UInt64, length: Int) throws -> Data {
        guard length > 0 else { return Data() }
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        try handle.seek(toOffset: offset)
        guard let data = try handle.read(upToCount: length), data.count == length else {
            throw URLError(.cannotDecodeContentData)
        }
        return data
    }
}

public actor ControlAPI: FileTransferAPI {
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

    public func uploadStatus(documentId: String) async throws -> UploadStatus {
        try await requestJSON(path: "/api/trip/\(token)/uploads/\(documentId)", method: "GET")
    }

    public func uploadPart(
        _ item: PendingMediaMetadata,
        offset: UInt64,
        total: UInt64,
        part: Data,
        sha256: String
    ) async throws -> UploadStatus {
        try await requestJSON(
            path: "/api/trip/\(token)/uploads/\(item.documentId)",
            method: "POST",
            body: part,
            headers: Self.uploadHeaders(item),
            query: [
                URLQueryItem(name: "offset", value: String(offset)),
                URLQueryItem(name: "total", value: String(total)),
                URLQueryItem(name: "part_sha256", value: sha256),
            ]
        )
    }

    public func uploadWhole(_ item: PendingMediaMetadata, file: URL) async throws {
        var request = makeRequest(
            path: "/api/trip/\(token)/documents/\(item.documentId)",
            method: "POST",
            accept: "application/json"
        )
        for (name, value) in Self.uploadHeaders(item) {
            request.setValue(value, forHTTPHeaderField: name)
        }
        let (data, response) = try await session.upload(for: request, fromFile: file)
        _ = try Self.accepted(data, response)
    }

    public func downloadPart(
        documentId: String,
        offset: UInt64,
        length: Int,
        destination: URL
    ) async throws -> Bool {
        let request = makeRequest(
            path: "/api/trip/\(token)/documents/\(documentId)",
            method: "GET",
            accept: "*/*",
            query: [
                URLQueryItem(name: "offset", value: String(offset)),
                URLQueryItem(name: "length", value: String(length)),
            ]
        )
        let (data, response) = try await session.data(for: request)
        let status = try Self.accepted(data, response)
        guard status == 206 else {
            // Control 0.3.0 and earlier sends the whole file.
            try data.write(to: destination, options: .atomic)
            return false
        }
        // A part cut short by the network is not appended.
        guard data.count == length else { throw URLError(.networkConnectionLost) }
        let handle = try FileHandle(forWritingTo: destination)
        defer { try? handle.close() }
        try handle.truncate(atOffset: offset)
        try handle.seek(toOffset: offset)
        try handle.write(contentsOf: data)
        return true
    }

    private static func uploadHeaders(_ item: PendingMediaMetadata) -> [String: String] {
        var headers = [
            "Content-Type": item.mimeType.isEmpty ? "application/octet-stream" : item.mimeType,
            "X-File-Name": percentEncodeHeader(item.originalName),
            "X-Created-At-Unix-Ms": String(item.createdAtUnixMs),
        ]
        if let pointId = item.pointId {
            headers["X-Point-Id"] = pointId
        }
        return headers
    }

    private func makeRequest(
        path: String,
        method: String,
        accept: String,
        query: [URLQueryItem] = []
    ) -> URLRequest {
        var url = baseURL.appending(path: path)
        if !query.isEmpty,
           var components = URLComponents(url: url, resolvingAgainstBaseURL: false) {
            components.queryItems = query
            url = components.url ?? url
        }
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue(accept, forHTTPHeaderField: "Accept")
        request.cachePolicy = .reloadIgnoringLocalAndRemoteCacheData
        return request
    }

    private static func accepted(_ data: Data, _ response: URLResponse) throws -> Int {
        guard let http = response as? HTTPURLResponse else {
            throw ControlAPIError.invalidResponse
        }
        guard (200...299).contains(http.statusCode) else {
            throw ControlAPIError.http(
                status: http.statusCode,
                body: String(data: data, encoding: .utf8) ?? ""
            )
        }
        return http.statusCode
    }

    private func requestJSON<T: Decodable>(
        path: String,
        method: String,
        body: Data? = nil,
        headers: [String: String] = [:],
        query: [URLQueryItem] = []
    ) async throws -> T {
        let data = try await requestData(
            path: path,
            method: method,
            body: body,
            headers: headers,
            accept: "application/json",
            query: query
        )
        return try decoder.decode(T.self, from: data)
    }

    private func requestData(
        path: String,
        method: String,
        body: Data? = nil,
        headers: [String: String] = [:],
        accept: String,
        query: [URLQueryItem] = []
    ) async throws -> Data {
        var request = makeRequest(path: path, method: method, accept: accept, query: query)
        request.httpBody = body
        for (name, value) in headers {
            request.setValue(value, forHTTPHeaderField: name)
        }
        let (data, response) = try await session.data(for: request)
        _ = try Self.accepted(data, response)
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
