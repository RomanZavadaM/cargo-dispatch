import Foundation
import Testing
@testable import DriverCore

private func temporaryRoot(_ name: String) -> URL {
    FileManager.default.temporaryDirectory
        .appending(path: "cargo-driver-\(name)-\(UUID().uuidString)", directoryHint: .isDirectory)
}

private func position(_ at: UInt64) -> LocationUpdate {
    LocationUpdate(latitude: 49.84, longitude: 24.03, accuracyM: 6, observedAtUnixMs: at)
}

private func accepted(_ update: LocationUpdate) -> DriverLocation {
    DriverLocation(
        latitude: update.latitude,
        longitude: update.longitude,
        accuracyM: update.accuracyM,
        observedAtUnixMs: update.observedAtUnixMs
    )
}

@Test
func positionsRecordedOfflineSurviveRestartInOrder() async throws {
    let root = temporaryRoot("queue")
    defer { try? FileManager.default.removeItem(at: root) }

    let queue = PendingOutboundQueue(root: root)
    try await queue.enqueueLocation(position(100))
    try await queue.enqueueLocation(position(200))
    try await queue.enqueueLocation(position(150))

    let reopened = PendingOutboundQueue(root: root)
    #expect(await reopened.pendingCount() == 2)
    #expect(await reopened.oldestLocation() == position(100))
    try await reopened.removeLocation(through: 100)
    #expect(await reopened.oldestLocation() == position(200))
    try await reopened.removeLocation(through: 200)
    #expect(await reopened.oldestLocation() == nil)
}

@Test
func theTrackIsBoundedAndDropsTheOldestPositions() async throws {
    let root = temporaryRoot("bounded")
    defer { try? FileManager.default.removeItem(at: root) }

    let queue = PendingOutboundQueue(root: root)
    for at in 1...UInt64(PendingOutboundQueue.maxTrack + 3) {
        try await queue.enqueueLocation(position(at))
    }
    #expect(await queue.pendingCount() == PendingOutboundQueue.maxTrack)
    #expect(await queue.oldestLocation() == position(4))
}

@Test
func aQueueWrittenByThePreviousVersionKeepsItsPosition() async throws {
    let root = temporaryRoot("legacy")
    defer { try? FileManager.default.removeItem(at: root) }
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    let json = """
    {"location":{"latitude":49.84,"longitude":24.03,"accuracy_m":6,"observed_at_unix_ms":77},"media":[]}
    """
    try Data(json.utf8).write(to: root.appending(path: "queue.json"))

    let queue = PendingOutboundQueue(root: root)
    #expect(await queue.oldestLocation() == position(77))
}

@Test
func sendingSkipsRefusedPositionsAndKeepsTheRestForLater() async throws {
    let root = temporaryRoot("sending")
    defer { try? FileManager.default.removeItem(at: root) }

    let queue = PendingOutboundQueue(root: root)
    for at in 1...UInt64(4) {
        try await queue.enqueueLocation(position(at))
    }

    let last = await OutboundSender().sendLocations(queue: queue) { update in
        switch update.observedAtUnixMs {
        case 2: throw ControlAPIError.http(status: 409, body: "older")
        case 3: throw URLError(.notConnectedToInternet)
        default: return accepted(update)
        }
    }

    #expect(last == accepted(position(1)))
    #expect(await queue.oldestLocation() == position(3))
    #expect(await queue.pendingCount() == 2)

    _ = await OutboundSender().sendLocations(queue: queue) { _ in
        throw ControlAPIError.http(status: 404, body: "gone")
    }
    #expect(await queue.pendingCount() == 2)
}

@Test
func mediaPayloadAndStableDocumentIdSurviveRestart() async throws {
    let root = temporaryRoot("media")
    defer { try? FileManager.default.removeItem(at: root) }

    let queue = PendingOutboundQueue(root: root)
    let documentId = "0123456789abcdef0123456789abcdef"
    let bytes = Data([1, 2, 3, 4])

    try await queue.enqueueMedia(
        documentId: documentId,
        originalName: "proof.jpg",
        mimeType: "image/jpeg",
        source: try temporaryFile(bytes),
        pointId: "51515151515151515151515151515151",
        createdAtUnixMs: 300
    )

    let reopened = PendingOutboundQueue(root: root)
    let pending = await reopened.pendingMedia()
    #expect(pending.count == 1)
    #expect(pending[0].documentId == documentId)
    #expect(pending[0].originalName == "proof.jpg")
    #expect(pending[0].mimeType == "image/jpeg")
    let kept = try #require(await reopened.mediaFile(pending[0]))
    #expect(try Data(contentsOf: kept) == bytes)
    #expect(await reopened.pendingCount() == 1)

    try await reopened.removeMedia(documentId: documentId)
    #expect(await reopened.pendingCount() == 0)
    #expect(await reopened.pendingMedia().isEmpty)
}

@Test
func aRefusedFileStaysVisibleUntilTheDriverDecides() async throws {
    let root = temporaryRoot("refused")
    defer { try? FileManager.default.removeItem(at: root) }

    let queue = PendingOutboundQueue(root: root)
    let documentId = "0123456789abcdef0123456789abcdef"
    try await queue.enqueueMedia(
        documentId: documentId,
        originalName: "big.mp4",
        mimeType: "video/mp4",
        source: try temporaryFile(Data([1])),
        createdAtUnixMs: 1
    )
    try await queue.markRejected(documentId: documentId, status: 413)

    let reopened = PendingOutboundQueue(root: root)
    #expect(await reopened.pendingCount() == 0)
    #expect(await reopened.pendingMedia().isEmpty)
    #expect(await reopened.rejectedMedia().first?.rejectedStatus == 413)

    try await reopened.retryRejected(documentId: documentId)
    #expect(await reopened.pendingCount() == 1)
    #expect(await reopened.rejectedMedia().isEmpty)
}

@Test
func sessionQueueDirectoryDoesNotExposeDriverToken() async {
    let base = temporaryRoot("session")
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

private func temporaryFile(_ bytes: Data) throws -> URL {
    let url = FileManager.default.temporaryDirectory
        .appending(path: "source-\(UUID().uuidString)")
    try bytes.write(to: url)
    return url
}
