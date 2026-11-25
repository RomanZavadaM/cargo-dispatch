import Foundation
import Testing
@testable import DriverCore

@Test
func newestPendingLocationWinsAndSurvivesRestart() async throws {
    let root = FileManager.default.temporaryDirectory
        .appending(path: "cargo-driver-queue-\(UUID().uuidString)", directoryHint: .isDirectory)
    defer { try? FileManager.default.removeItem(at: root) }

    let queue = PendingOutboundQueue(root: root)
    let newer = LocationUpdate(
        latitude: 49.84,
        longitude: 24.03,
        accuracyM: 6,
        observedAtUnixMs: 200
    )
    try await queue.enqueueLocation(newer)
    try await queue.enqueueLocation(
        LocationUpdate(
            latitude: 49.83,
            longitude: 24.02,
            accuracyM: 8,
            observedAtUnixMs: 100
        )
    )

    #expect(await queue.pendingLocation() == newer)
    #expect(await queue.pendingCount() == 1)

    let reopened = PendingOutboundQueue(root: root)
    #expect(await reopened.pendingLocation() == newer)

    try await reopened.removeLocation(through: 199)
    #expect(await reopened.pendingLocation() == newer)
    try await reopened.removeLocation(through: 200)
    #expect(await reopened.pendingLocation() == nil)
}

@Test
func mediaPayloadAndStableDocumentIdSurviveRestart() async throws {
    let root = FileManager.default.temporaryDirectory
        .appending(path: "cargo-driver-media-\(UUID().uuidString)", directoryHint: .isDirectory)
    defer { try? FileManager.default.removeItem(at: root) }

    let queue = PendingOutboundQueue(root: root)
    let documentId = "0123456789abcdef0123456789abcdef"
    let bytes = Data([1, 2, 3, 4])

    try await queue.enqueueMedia(
        documentId: documentId,
        originalName: "proof.jpg",
        mimeType: "image/jpeg",
        data: bytes,
        pointId: "51515151515151515151515151515151",
        createdAtUnixMs: 300
    )

    let reopened = PendingOutboundQueue(root: root)
    let pending = await reopened.pendingMedia()
    #expect(pending.count == 1)
    #expect(pending[0].metadata.documentId == documentId)
    #expect(pending[0].metadata.originalName == "proof.jpg")
    #expect(pending[0].metadata.mimeType == "image/jpeg")
    #expect(pending[0].data == bytes)
    #expect(await reopened.pendingCount() == 1)

    try await reopened.removeMedia(documentId: documentId)
    #expect(await reopened.pendingCount() == 0)
    #expect(await reopened.pendingMedia().isEmpty)
}

@Test
func sessionQueueDirectoryDoesNotExposeDriverToken() async {
    let base = FileManager.default.temporaryDirectory
        .appending(path: "cargo-driver-session-\(UUID().uuidString)", directoryHint: .isDirectory)
    let token = "0123456789abcdef0123456789abcdef"
    let queue = PendingOutboundQueue.forSession(
        baseRoot: base,
        baseURL: URL(string: "http://127.0.0.1:8765")!,
        token: token
    )

    #expect(await queue.pendingCount() == 0)
    let names = (try? FileManager.default.contentsOfDirectory(atPath: base.path)) ?? []
    #expect(!names.contains(where: { $0.contains(token) }))
}
