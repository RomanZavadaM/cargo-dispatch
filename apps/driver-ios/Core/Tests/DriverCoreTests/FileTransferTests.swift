import Foundation
import Testing
@testable import DriverCore

private let content = Data((0..<2_500).map { UInt8($0 % 251) })

private let item = PendingMediaMetadata(
    documentId: "0123456789abcdef0123456789abcdef",
    originalName: "video.mp4",
    mimeType: "video/mp4",
    pointId: nil,
    createdAtUnixMs: 1,
    payloadFile: "video.bin"
)

private actor FakeControl: FileTransferAPI {
    private let document: Data
    private let old: Bool
    private var failAt: UInt64?
    private var total: UInt64 = .max
    private(set) var held = Data()
    private(set) var offsets: [UInt64] = []
    private(set) var wholeUploads = 0

    init(document: Data = Data(), old: Bool = false, failAt: UInt64? = nil) {
        self.document = document
        self.old = old
        self.failAt = failAt
    }

    func uploadStatus(documentId: String) async throws -> UploadStatus {
        if old { throw ControlAPIError.http(status: 404, body: "no such route") }
        return UploadStatus(received: UInt64(held.count), complete: UInt64(held.count) == total)
    }

    func uploadPart(
        _ item: PendingMediaMetadata,
        offset: UInt64,
        total: UInt64,
        part: Data,
        sha256: String
    ) async throws -> UploadStatus {
        try breakOnce(at: offset)
        #expect(sha256 == FileTransfer.sha256Hex(part))
        guard offset == UInt64(held.count) else {
            throw ControlAPIError.http(status: 409, body: "gap")
        }
        offsets.append(offset)
        self.total = total
        held.append(part)
        return UploadStatus(received: UInt64(held.count), complete: UInt64(held.count) == total)
    }

    func uploadWhole(_ item: PendingMediaMetadata, file: URL) async throws {
        wholeUploads += 1
        held = try Data(contentsOf: file)
    }

    func downloadPart(
        documentId: String,
        offset: UInt64,
        length: Int,
        destination: URL
    ) async throws -> Bool {
        try breakOnce(at: offset)
        if old {
            try document.write(to: destination)
            return false
        }
        offsets.append(offset)
        let handle = try FileHandle(forWritingTo: destination)
        defer { try? handle.close() }
        try handle.seek(toOffset: offset)
        try handle.write(contentsOf: document.subdata(in: Int(offset)..<Int(offset) + length))
        return true
    }

    private func breakOnce(at offset: UInt64) throws {
        if failAt == offset {
            failAt = nil
            throw URLError(.networkConnectionLost)
        }
    }
}

private func scratch() -> URL {
    FileManager.default.temporaryDirectory
        .appending(path: "transfer-\(UUID().uuidString)")
}

@Test
func aBrokenUploadContinuesWhereItStopped() async throws {
    let control = FakeControl(failAt: 1_000)
    let file = scratch()
    try content.write(to: file)

    await #expect(throws: URLError.self) {
        try await FileTransfer.upload(api: control, item: item, file: file, partBytes: 1_000)
    }
    try await FileTransfer.upload(api: control, item: item, file: file, partBytes: 1_000)

    #expect(await control.offsets == [0, 1_000, 2_000])
    #expect(await control.held == content)
}

@Test
func anOlderControlReceivesTheFileWhole() async throws {
    let control = FakeControl(old: true)
    let file = scratch()
    try content.write(to: file)

    try await FileTransfer.upload(api: control, item: item, file: file, partBytes: 1_000)

    #expect(await control.wholeUploads == 1)
    #expect(await control.held == content)
}

@Test
func aBrokenDownloadContinuesAndIsChecked() async throws {
    let control = FakeControl(document: content, failAt: 1_000)
    let partial = scratch().appending(path: "document.part")
    let checksum = FileTransfer.sha256Hex(content)

    await #expect(throws: URLError.self) {
        try await FileTransfer.download(
            api: control, documentId: "id", size: UInt64(content.count),
            sha256: checksum, partial: partial, partBytes: 1_000
        )
    }
    try await FileTransfer.download(
        api: control, documentId: "id", size: UInt64(content.count),
        sha256: checksum, partial: partial, partBytes: 1_000
    )

    #expect(await control.offsets == [0, 1_000, 2_000])
    #expect(try Data(contentsOf: partial) == content)
}

@Test
func aDownloadWithAnotherChecksumIsThrownAway() async throws {
    let partial = scratch().appending(path: "document.part")

    await #expect(throws: URLError.self) {
        try await FileTransfer.download(
            api: FakeControl(document: content), documentId: "id",
            size: UInt64(content.count), sha256: String(repeating: "0", count: 64),
            partial: partial, partBytes: 1_000
        )
    }

    #expect(!FileManager.default.fileExists(atPath: partial.path))
}
