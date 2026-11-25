import PhotosUI
import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @StateObject private var store = DriverStore()
    @StateObject private var locationController = ForegroundLocationController()

    @State private var showFileImporter = false
    @State private var mediaSelection: PhotosPickerItem?
    @State private var exportDocument = DriverFileDocument()
    @State private var exportName = "document"
    @State private var showFileExporter = false

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Text("screen.subtitle")
                        .font(.subheadline)
                }

                Section("connection.title") {
                    TextField("driver_url.label", text: $store.driverURL)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.URL)

                    Button(store.busy ? "connecting" : "connect") {
                        store.connect()
                    }
                    .disabled(store.busy || store.driverURL.isEmpty)

                    if store.trip != nil {
                        Button("refresh") {
                            store.refresh()
                        }
                        .disabled(store.busy)

                        Button("disconnect") {
                            store.disconnect()
                        }
                        .disabled(store.busy)
                    }
                }

                if let errorKey = store.errorKey {
                    Section {
                        Text(LocalizedStringKey(errorKey))
                            .foregroundStyle(.red)
                    }
                }

                if let trip = store.trip {
                    Section {
                        Text(
                            String(
                                format: NSLocalizedString("trip.title.format", comment: ""),
                                String(trip.tripId.prefix(8))
                            )
                        )
                        Text(
                            String(
                                format: NSLocalizedString("trip.version_revision.format", comment: ""),
                                trip.tripVersion,
                                trip.localRouteRevision
                            )
                        )
                        Text(
                            String(
                                format: NSLocalizedString("trip.points_documents.format", comment: ""),
                                trip.points.count,
                                trip.documents.count
                            )
                        )
                        if let geometry = trip.roadGeometry {
                            Text(
                                String(
                                    format: NSLocalizedString("trip.geometry.format", comment: ""),
                                    geometry.points.count,
                                    geometry.routing.engine,
                                    geometry.routing.engineVersion
                                )
                            )
                        } else {
                            Text("trip.geometry.none")
                        }
                    }

                    Section("location.title") {
                        if let location = trip.latestLocation {
                            Text(
                                String(
                                    format: NSLocalizedString("location.value.format", comment: ""),
                                    location.latitude,
                                    location.longitude,
                                    location.accuracyM ?? 0
                                )
                            )
                            .font(.caption)
                        } else {
                            Text("location.none")
                        }

                        Button("location.send") {
                            Task {
                                do {
                                    store.sendLocation(try await locationController.current())
                                } catch {
                                    store.reportError("error.location")
                                }
                            }
                        }
                        .disabled(store.busy)
                    }

                    Section("documents.title") {
                        if trip.documents.isEmpty {
                            Text("documents.none")
                        } else {
                            ForEach(trip.documents) { document in
                                VStack(alignment: .leading, spacing: 6) {
                                    Text(document.originalName)
                                        .font(.headline)
                                    Text("\(document.size) B · \(document.mimeType)")
                                        .font(.caption)
                                    Button("documents.save") {
                                        store.download(document: document) { result in
                                            if case let .success(data) = result {
                                                exportDocument = DriverFileDocument(data: data)
                                                exportName = document.originalName
                                                showFileExporter = true
                                            }
                                        }
                                    }
                                    .disabled(store.busy)
                                }
                            }
                        }

                        Button("documents.upload_file") {
                            showFileImporter = true
                        }
                        .disabled(store.busy)

                        PhotosPicker(
                            selection: $mediaSelection,
                            matching: .any(of: [.images, .videos])
                        ) {
                            Text("documents.upload_media")
                        }
                        .disabled(store.busy)
                    }

                    Section("route.title") {
                        ForEach(trip.points) { point in
                            VStack(alignment: .leading, spacing: 8) {
                                Text("\(point.sequence). \(point.title)")
                                    .font(.headline)

                                Text(point.completed ? "point.completed" : "point.open")
                                    .font(.subheadline)

                                if let navigation = point.navigation {
                                    Text(
                                        navigation.address
                                            ?? "\(navigation.latitude), \(navigation.longitude)"
                                    )
                                    .font(.caption)
                                }

                                if !point.completed {
                                    HStack {
                                        Button("move.up") {
                                            store.moveOpen(pointId: point.pointId, delta: -1)
                                        }
                                        Button("move.down") {
                                            store.moveOpen(pointId: point.pointId, delta: 1)
                                        }
                                        Button("complete") {
                                            store.complete(pointId: point.pointId)
                                        }
                                    }
                                    .buttonStyle(.bordered)
                                    .disabled(store.busy)
                                }
                            }
                            .padding(.vertical, 4)
                        }
                    }
                }
            }
            .navigationTitle("screen.title")
            .navigationBarTitleDisplayMode(.inline)
            .fileImporter(
                isPresented: $showFileImporter,
                allowedContentTypes: [.data],
                allowsMultipleSelection: false
            ) { result in
                do {
                    guard let url = try result.get().first else { return }
                    let scoped = url.startAccessingSecurityScopedResource()
                    defer {
                        if scoped {
                            url.stopAccessingSecurityScopedResource()
                        }
                    }
                    let values = try? url.resourceValues(forKeys: [.contentTypeKey])
                    store.upload(
                        data: try Data(contentsOf: url),
                        name: url.lastPathComponent,
                        mimeType: values?.contentType?.preferredMIMEType ?? "application/octet-stream"
                    )
                } catch {
                    store.reportError("error.file")
                }
            }
            .fileExporter(
                isPresented: $showFileExporter,
                document: exportDocument,
                contentType: .data,
                defaultFilename: exportName
            ) { result in
                if case .failure = result {
                    store.reportError("error.file")
                }
            }
            .onChange(of: mediaSelection) { _, item in
                guard let item else { return }
                Task {
                    do {
                        guard let data = try await item.loadTransferable(type: Data.self) else {
                            store.reportError("error.file")
                            return
                        }
                        let type = item.supportedContentTypes.first
                        let suffix = type?.preferredFilenameExtension ?? "bin"
                        store.upload(
                            data: data,
                            name: "driver-media.\(suffix)",
                            mimeType: type?.preferredMIMEType ?? "application/octet-stream"
                        )
                        mediaSelection = nil
                    } catch {
                        store.reportError("error.file")
                    }
                }
            }
        }
    }
}
