import PhotosUI
import SwiftUI
import UniformTypeIdentifiers
import UIKit

private let refreshInterval: Duration = .seconds(15)

struct ContentView: View {
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.openURL) private var openURL
    @StateObject private var store = DriverStore()
    @StateObject private var locationController = ForegroundLocationController()
    @StateObject private var trackingController = BackgroundTrackingController()

    @State private var showFileImporter = false
    @State private var mediaSelection: PhotosPickerItem?
    @State private var exportDocument = DriverFileDocument()
    @State private var exportName = "document"
    @State private var showFileExporter = false
    @State private var cameraMode: DriverCameraMode?
    @State private var attachPointId: String?
    @State private var navigationPoint: DriverPoint?
    @State private var confirmPoint: DriverPoint?

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Text("screen.subtitle")
                        .font(.subheadline)
                }

                connectionSection

                if let problem = store.problem {
                    Section {
                        Text(LocalizedStringKey(problem.textKey))
                            .foregroundStyle(.red)
                    }
                }

                if !store.rejected.isEmpty {
                    rejectedSection
                }

                if let trip = store.trip {
                    tripSection(trip)
                    mapSection(trip)
                    locationSection(trip)
                    documentsSection(trip)
                    routeSection(trip)
                }
            }
            .navigationTitle("screen.title")
            .navigationBarTitleDisplayMode(.inline)
            .fileImporter(
                isPresented: $showFileImporter,
                allowedContentTypes: [.data],
                allowsMultipleSelection: false
            ) { result in
                importFile(result)
            }
            .fileExporter(
                isPresented: $showFileExporter,
                document: exportDocument,
                contentType: .data,
                defaultFilename: exportName
            ) { result in
                if case .failure = result {
                    store.report(.fileNotSaved)
                }
            }
            .onChange(of: mediaSelection) { _, item in
                importMedia(item)
            }
            .sheet(item: $cameraMode) { mode in
                DriverCameraCaptureView(
                    mode: mode,
                    onResult: { result in
                        cameraMode = nil
                        switch result {
                        case let .success(media):
                            store.upload(
                                file: media.file,
                                name: media.name,
                                mimeType: media.mimeType,
                                pointId: attachPointId
                            )
                        case .failure:
                            store.report(.camera)
                        }
                    },
                    onCancel: {
                        cameraMode = nil
                    }
                )
            }
            .confirmationDialog(
                "navigate",
                isPresented: presented($navigationPoint),
                presenting: navigationPoint
            ) { point in
                navigationButtons(point)
            }
            .alert(
                "confirm.done.title",
                isPresented: presented($confirmPoint),
                presenting: confirmPoint
            ) { point in
                Button("confirm.done.button") {
                    store.complete(pointId: point.pointId)
                }
                Button("cancel", role: .cancel) {}
            } message: { point in
                Text(
                    String(
                        format: NSLocalizedString("confirm.done.format", comment: ""),
                        point.sequence,
                        point.title
                    )
                )
            }
            // New points, documents and the dispatcher's changes appear by themselves.
            .task(id: scenePhase) {
                guard scenePhase == .active else { return }
                while !Task.isCancelled {
                    await store.refreshQuietly()
                    try? await Task.sleep(for: refreshInterval)
                }
            }
        }
    }

    private var connectionSection: some View {
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
                Text(store.offline ? "status.offline" : "status.connected")
                    .font(.caption)
                    .foregroundStyle(store.offline ? Color.red : Color.secondary)

                if store.pendingOutbound > 0 {
                    Text(
                        String(
                            format: NSLocalizedString("queue.pending.format", comment: ""),
                            store.pendingOutbound
                        )
                    )
                    .font(.caption)
                    .foregroundStyle(.orange)
                }

                Button("refresh") {
                    store.refresh()
                }
                .disabled(store.busy)

                Button("disconnect") {
                    trackingController.stop()
                    store.disconnect()
                }
                .disabled(store.busy)
            }
        }
    }

    private var rejectedSection: some View {
        Section("rejected.title") {
            ForEach(store.rejected) { item in
                VStack(alignment: .leading, spacing: 6) {
                    Text(item.originalName)
                        .font(.headline)
                    Text(
                        LocalizedStringKey(
                            DriverProblem.fromStatus(item.rejectedStatus ?? 400).textKey
                        )
                    )
                    .font(.caption)
                    HStack {
                        Button("retry") {
                            store.retryRejected(documentId: item.documentId)
                        }
                        Button("discard", role: .destructive) {
                            store.discardRejected(documentId: item.documentId)
                        }
                    }
                    .buttonStyle(.bordered)
                    .disabled(store.busy)
                }
            }
        }
    }

    private func tripSection(_ trip: DriverTrip) -> some View {
        Section {
            Text(
                String(
                    format: NSLocalizedString("trip.title.format", comment: ""),
                    String(trip.tripId.prefix(8))
                )
            )
            .font(.headline)
            Text(
                String(
                    format: NSLocalizedString("trip.progress.format", comment: ""),
                    trip.points.filter(\.completed).count,
                    trip.points.count
                )
            )
        }
    }

    private func mapSection(_ trip: DriverTrip) -> some View {
        Section("map.title") {
            DriverTripMap(trip: trip)
            Text("map.note")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private func locationSection(_ trip: DriverTrip) -> some View {
        Section("location.title") {
            if let location = trip.latestLocation {
                Text(
                    String(
                        format: NSLocalizedString("location.value.format", comment: ""),
                        Date(timeIntervalSince1970: Double(location.observedAtUnixMs) / 1000)
                            .formatted(date: .numeric, time: .shortened),
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
                        store.report(.locationUnavailable)
                    }
                }
            }
            .disabled(store.busy)

            if trackingController.isTracking {
                Text("location.tracking.active")
                    .font(.caption)
                    .foregroundStyle(.green)

                Button("location.tracking.stop", role: .destructive) {
                    trackingController.stop()
                }
            } else {
                Button("location.tracking.start") {
                    trackingController.start(
                        onUpdate: { update in
                            store.trackLocation(update)
                        },
                        onFailure: {
                            store.report(.locationUnavailable)
                        }
                    )
                }
                .disabled(store.busy)
            }

            Text("location.tracking.note")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private func documentsSection(_ trip: DriverTrip) -> some View {
        Section("documents.title") {
            if trip.documents.isEmpty {
                Text("documents.none")
            } else {
                ForEach(trip.documents) { document in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(document.originalName)
                            .font(.headline)
                        Text(verbatim: details(of: document, in: trip))
                            .font(.caption)
                        Button("documents.save") {
                            store.download(document: document) { file in
                                exportDocument = DriverFileDocument(url: file)
                                exportName = document.originalName
                                showFileExporter = true
                            }
                        }
                        .buttonStyle(.bordered)
                        .disabled(store.busy)
                    }
                }
            }

            Picker("attach.to", selection: $attachPointId) {
                Text("attach.trip").tag(String?.none)
                ForEach(trip.points) { point in
                    Text(verbatim: label(of: point)).tag(String?.some(point.pointId))
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

            if UIImagePickerController.isSourceTypeAvailable(.camera) {
                Button("documents.capture_photo") {
                    cameraMode = .photo
                }
                .disabled(store.busy)

                Button("documents.capture_video") {
                    cameraMode = .video
                }
                .disabled(store.busy)
            } else {
                Text("camera.unavailable")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func routeSection(_ trip: DriverTrip) -> some View {
        Section("route.title") {
            ForEach(trip.points) { point in
                pointRow(point)
            }
        }
    }

    private func pointRow(_ point: DriverPoint) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(verbatim: label(of: point))
                .font(.headline)

            Text(point.completed ? "point.completed" : "point.open")
                .font(.subheadline)

            if let navigation = point.navigation {
                Text(
                    verbatim: navigation.address
                        ?? "\(navigation.latitude), \(navigation.longitude)"
                )
                .font(.caption)
            }

            if !point.completed {
                if point.navigation != nil {
                    Button("navigate") {
                        navigationPoint = point
                    }
                    .buttonStyle(.borderedProminent)
                }

                Button("complete") {
                    confirmPoint = point
                }
                .buttonStyle(.bordered)
                .disabled(store.busy)

                HStack {
                    Button("move.up") {
                        store.moveOpen(pointId: point.pointId, delta: -1)
                    }
                    Button("move.down") {
                        store.moveOpen(pointId: point.pointId, delta: 1)
                    }
                }
                .buttonStyle(.bordered)
                .disabled(store.busy)
            }
        }
        .padding(.vertical, 4)
    }

    @ViewBuilder
    private func navigationButtons(_ point: DriverPoint) -> some View {
        if let destination = point.navigation {
            Button("navigate.apple") {
                if let url = NavigationLinks.appleMaps(
                    latitude: destination.latitude,
                    longitude: destination.longitude
                ) {
                    openURL(url)
                }
            }
            Button("navigate.google") {
                if let url = NavigationLinks.googleMaps(
                    latitude: destination.latitude,
                    longitude: destination.longitude
                ) {
                    openURL(url)
                }
            }
        }
        Button("cancel", role: .cancel) {}
    }

    private func presented(_ point: Binding<DriverPoint?>) -> Binding<Bool> {
        Binding(
            get: { point.wrappedValue != nil },
            set: { shown in
                if !shown {
                    point.wrappedValue = nil
                }
            }
        )
    }

    private func label(of point: DriverPoint) -> String {
        "\(point.sequence). \(point.title)"
    }

    private func details(of document: DriverDocument, in trip: DriverTrip) -> String {
        var parts = [Int64(clamping: document.size).formatted(.byteCount(style: .file))]
        if let point = trip.points.first(where: { $0.pointId == document.pointId }) {
            parts.append(label(of: point))
        }
        return parts.joined(separator: " · ")
    }

    private func importFile(_ result: Result<[URL], Error>) {
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
                file: try DriverStaging.copy(url),
                name: url.lastPathComponent,
                mimeType: values?.contentType?.preferredMIMEType ?? "application/octet-stream",
                pointId: attachPointId
            )
        } catch {
            store.report(.fileUnreadable)
        }
    }

    private func importMedia(_ item: PhotosPickerItem?) {
        guard let item else { return }
        Task {
            do {
                guard let picked = try await item.loadTransferable(type: PickedMediaFile.self) else {
                    store.report(.fileUnreadable)
                    return
                }
                let type = item.supportedContentTypes.first
                let suffix = type?.preferredFilenameExtension ?? "bin"
                store.upload(
                    file: picked.url,
                    name: "driver-media.\(suffix)",
                    mimeType: type?.preferredMIMEType ?? "application/octet-stream",
                    pointId: attachPointId
                )
                mediaSelection = nil
            } catch {
                store.report(.fileUnreadable)
            }
        }
    }
}
