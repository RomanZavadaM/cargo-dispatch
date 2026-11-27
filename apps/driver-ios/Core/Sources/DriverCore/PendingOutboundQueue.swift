import CryptoKit
import Foundation

public struct PendingMediaMetadata: Codable, Equatable, Identifiable, Sendable {
    public let documentId: String
    public let originalName: String
    public let mimeType: String
    public let pointId: String?
    public let createdAtUnixMs: UInt64
    public let payloadFile: String
    /// HTTP status of a refusal that is not retried until the driver asks.
    public var rejectedStatus: Int?

    public var id: String { documentId }

    public init(
        documentId: String,
        originalName: String,
        mimeType: String,
        pointId: String?,
        createdAtUnixMs: UInt64,
        payloadFile: String,
        rejectedStatus: Int? = nil
    ) {
        self.documentId = documentId
        self.originalName = originalName
        self.mimeType = mimeType
        self.pointId = pointId
        self.createdAtUnixMs = createdAtUnixMs
        self.payloadFile = payloadFile
        self.rejectedStatus = rejectedStatus
    }
}

private struct PendingOutboundState: Codable, Equatable, Sendable {
    // Written by 0.2.0 and earlier: only the newest position. Read once and moved to `track`.
    var location: LocationUpdate?
    var track: [LocationUpdate]?
    var media: [PendingMediaMetadata]

    static let empty = PendingOutboundState(location: nil, track: [], media: [])
}

public actor PendingOutboundQueue {
    /// About two hours of tracking at one position every 15 seconds.
    public static let maxTrack = 500

    private let root: URL
    private let metadataURL: URL
    private let mediaDirectory: URL
    private let encoder: JSONEncoder
    private let decoder: JSONDecoder
    private var state: PendingOutboundState

    public init(root: URL) {
        self.root = root
        self.metadataURL = root.appending(path: "queue.json")
        self.mediaDirectory = root.appending(path: "media", directoryHint: .isDirectory)

        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        self.encoder = encoder
        self.decoder = JSONDecoder()

        var restored = PendingOutboundState.empty
        if let data = try? Data(contentsOf: metadataURL),
           let saved = try? decoder.decode(PendingOutboundState.self, from: data) {
            restored = saved
        }
        if let legacy = restored.location {
            restored.location = nil
            restored.track = ((restored.track ?? []) + [legacy])
                .sorted { $0.observedAtUnixMs < $1.observedAtUnixMs }
        }
        self.state = restored
    }

    public static func forSession(
        baseRoot: URL,
        baseURL: URL,
        token: String
    ) -> PendingOutboundQueue {
        let normalized = baseURL.absoluteString.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        let digest = SHA256.hash(data: Data("\(normalized)|\(token)".utf8))
        let key = digest.map { String(format: "%02x", $0) }.joined()
        return PendingOutboundQueue(root: baseRoot.appending(path: key, directoryHint: .isDirectory))
    }

    /// Keeps the positions recorded without a connection, oldest first.
    public func enqueueLocation(_ update: LocationUpdate) throws {
        var track = state.track ?? []
        if let newest = track.last, update.observedAtUnixMs <= newest.observedAtUnixMs {
            return
        }
        track.append(update)
        state.track = Array(track.suffix(Self.maxTrack))
        try persistState()
    }

    public func oldestLocation() -> LocationUpdate? {
        state.track?.first
    }

    public func removeLocation(through observedAtUnixMs: UInt64) throws {
        let track = state.track ?? []
        let remaining = track.filter { $0.observedAtUnixMs > observedAtUnixMs }
        guard remaining.count != track.count else { return }
        state.track = remaining
        try persistState()
    }

    @discardableResult
    public func enqueueMedia(
        documentId: String,
        originalName: String,
        mimeType: String,
        data: Data,
        pointId: String? = nil,
        createdAtUnixMs: UInt64
    ) throws -> PendingMediaMetadata {
        guard documentId.range(
            of: #"^[0-9a-f]{32}$"#,
            options: .regularExpression
        ) != nil else {
            throw PendingOutboundQueueError.invalidDocumentId
        }

        if let existing = state.media.first(where: { $0.documentId == documentId }) {
            return existing
        }

        try FileManager.default.createDirectory(
            at: mediaDirectory,
            withIntermediateDirectories: true
        )

        let payloadName = "\(documentId).bin"
        let payloadURL = mediaDirectory.appending(path: payloadName)
        try data.write(to: payloadURL, options: .atomic)

        let metadata = PendingMediaMetadata(
            documentId: documentId,
            originalName: originalName,
            mimeType: mimeType,
            pointId: pointId,
            createdAtUnixMs: createdAtUnixMs,
            payloadFile: payloadName
        )
        state.media.append(metadata)
        do {
            try persistState()
        } catch {
            try? FileManager.default.removeItem(at: payloadURL)
            state.media.removeAll { $0.documentId == documentId }
            throw error
        }
        return metadata
    }

    /// Files still to be sent. Their content is read one at a time by `readMedia`.
    public func pendingMedia() -> [PendingMediaMetadata] {
        state.media.filter { $0.rejectedStatus == nil }
    }

    public func rejectedMedia() -> [PendingMediaMetadata] {
        state.media.filter { $0.rejectedStatus != nil }
    }

    public func readMedia(_ item: PendingMediaMetadata) -> Data? {
        try? Data(contentsOf: mediaDirectory.appending(path: item.payloadFile))
    }

    public func markRejected(documentId: String, status: Int) throws {
        try setRejection(documentId: documentId, status: status)
    }

    public func retryRejected(documentId: String) throws {
        try setRejection(documentId: documentId, status: nil)
    }

    private func setRejection(documentId: String, status: Int?) throws {
        guard let index = state.media.firstIndex(where: { $0.documentId == documentId }) else {
            return
        }
        state.media[index].rejectedStatus = status
        try persistState()
    }

    public func removeMedia(documentId: String) throws {
        guard let existing = state.media.first(where: { $0.documentId == documentId }) else {
            return
        }
        state.media.removeAll { $0.documentId == documentId }
        try persistState()
        try? FileManager.default.removeItem(
            at: mediaDirectory.appending(path: existing.payloadFile)
        )
    }

    /// Items waiting to be sent; refused files are listed separately.
    public func pendingCount() -> Int {
        pendingMedia().count + (state.track?.count ?? 0)
    }

    private func persistState() throws {
        try FileManager.default.createDirectory(
            at: root,
            withIntermediateDirectories: true
        )
        try encoder.encode(state).write(to: metadataURL, options: .atomic)
    }
}

public enum PendingOutboundQueueError: Error, Equatable {
    case invalidDocumentId
}

/// Sends what waits in the queue. Only one round of each kind runs at a time, and
/// a large file does not hold up positions.
public actor OutboundSender {
    private var sendingLocations = false
    private var sendingMedia = false

    public init() {}

    /// Sends positions oldest first and stops at the first failure that a later
    /// attempt can fix. Returns the last accepted position.
    @discardableResult
    public func sendLocations(
        queue: PendingOutboundQueue,
        send: @Sendable (LocationUpdate) async throws -> DriverLocation
    ) async -> DriverLocation? {
        guard !sendingLocations else { return nil }
        sendingLocations = true
        defer { sendingLocations = false }

        var accepted: DriverLocation?
        while let next = await queue.oldestLocation() {
            do {
                accepted = try await send(next)
            } catch let error as ControlAPIError {
                guard case let .http(status, _) = error,
                      DriverProblem.isPermanentRejection(status) else { break }
            } catch {
                break
            }
            do {
                try await queue.removeLocation(through: next.observedAtUnixMs)
            } catch {
                break
            }
        }
        return accepted
    }

    public func sendMedia(client: ControlAPI, queue: PendingOutboundQueue) async {
        guard !sendingMedia else { return }
        sendingMedia = true
        defer { sendingMedia = false }

        for item in await queue.pendingMedia() {
            guard let data = await queue.readMedia(item) else {
                try? await queue.removeMedia(documentId: item.documentId)
                continue
            }
            do {
                _ = try await client.uploadDocument(
                    documentId: item.documentId,
                    originalName: item.originalName,
                    mimeType: item.mimeType,
                    data: data,
                    pointId: item.pointId,
                    createdAtUnixMs: item.createdAtUnixMs
                )
                try await queue.removeMedia(documentId: item.documentId)
            } catch let error as ControlAPIError {
                guard case let .http(status, _) = error,
                      DriverProblem.isPermanentRejection(status) else { return }
                try? await queue.markRejected(documentId: item.documentId, status: status)
            } catch {
                return
            }
        }
    }
}
