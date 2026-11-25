import CryptoKit
import Foundation

public struct PendingMediaMetadata: Codable, Equatable, Sendable {
    public let documentId: String
    public let originalName: String
    public let mimeType: String
    public let pointId: String?
    public let createdAtUnixMs: UInt64
    public let payloadFile: String

    public init(
        documentId: String,
        originalName: String,
        mimeType: String,
        pointId: String?,
        createdAtUnixMs: UInt64,
        payloadFile: String
    ) {
        self.documentId = documentId
        self.originalName = originalName
        self.mimeType = mimeType
        self.pointId = pointId
        self.createdAtUnixMs = createdAtUnixMs
        self.payloadFile = payloadFile
    }
}

public struct QueuedMediaUpload: Equatable, Sendable {
    public let metadata: PendingMediaMetadata
    public let data: Data
}

private struct PendingOutboundState: Codable, Equatable, Sendable {
    var location: LocationUpdate?
    var media: [PendingMediaMetadata]

    static let empty = PendingOutboundState(location: nil, media: [])
}

public actor PendingOutboundQueue {
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

        if let data = try? Data(contentsOf: metadataURL),
           let restored = try? decoder.decode(PendingOutboundState.self, from: data) {
            self.state = restored
        } else {
            self.state = .empty
        }
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

    public func enqueueLocation(_ update: LocationUpdate) throws {
        if let current = state.location,
           update.observedAtUnixMs < current.observedAtUnixMs {
            return
        }
        state.location = update
        try persistState()
    }

    public func pendingLocation() -> LocationUpdate? {
        state.location
    }

    public func removeLocation(through observedAtUnixMs: UInt64) throws {
        guard let current = state.location else { return }
        guard current.observedAtUnixMs <= observedAtUnixMs else { return }
        state.location = nil
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

    public func pendingMedia() -> [QueuedMediaUpload] {
        state.media.compactMap { item in
            let url = mediaDirectory.appending(path: item.payloadFile)
            guard let data = try? Data(contentsOf: url) else { return nil }
            return QueuedMediaUpload(metadata: item, data: data)
        }
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

    public func pendingCount() -> Int {
        state.media.count + (state.location == nil ? 0 : 1)
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
