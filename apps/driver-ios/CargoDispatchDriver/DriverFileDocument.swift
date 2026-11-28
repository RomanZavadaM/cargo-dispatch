import CoreTransferable
import SwiftUI
import UniformTypeIdentifiers

/// A received file offered to the system "save" window straight from disk.
struct DriverFileDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.data] }

    var url: URL?

    init(url: URL? = nil) {
        self.url = url
    }

    init(configuration: ReadConfiguration) throws {
        self.url = nil
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        guard let url else { return FileWrapper(regularFileWithContents: Data()) }
        return try FileWrapper(url: url)
    }
}

/// Own copies of chosen files: the originals may disappear before they are sent.
enum DriverStaging {
    static func newFile() -> URL {
        FileManager.default.temporaryDirectory
            .appending(path: "driver-upload-\(UUID().uuidString)")
    }

    static func copy(_ source: URL) throws -> URL {
        let target = newFile()
        try FileManager.default.copyItem(at: source, to: target)
        return target
    }

    static func write(_ data: Data) throws -> URL {
        let target = newFile()
        try data.write(to: target, options: .atomic)
        return target
    }
}

/// A photo or video from the library, received as a file rather than as bytes.
struct PickedMediaFile: Transferable {
    let url: URL

    static var transferRepresentation: some TransferRepresentation {
        FileRepresentation(importedContentType: .movie) { received in
            PickedMediaFile(url: try DriverStaging.copy(received.file))
        }
        FileRepresentation(importedContentType: .image) { received in
            PickedMediaFile(url: try DriverStaging.copy(received.file))
        }
    }
}
