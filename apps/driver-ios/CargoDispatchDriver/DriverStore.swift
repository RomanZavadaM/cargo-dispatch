import Combine
import Foundation

@MainActor
final class DriverStore: ObservableObject {
    @Published var driverURL: String
    @Published var trip: DriverTrip?
    @Published var busy = false
    @Published var pendingOutbound = 0
    @Published var errorKey: String?

    private var api: ControlAPI?
    private var pendingQueue: PendingOutboundQueue?
    private let defaults: UserDefaults
    private let queueBaseRoot: URL
    private let savedDriverURLKey = "cargo_dispatch_driver_url"

    init(
        defaults: UserDefaults = .standard,
        queueBaseRoot: URL? = nil
    ) {
        self.defaults = defaults
        self.driverURL = defaults.string(forKey: savedDriverURLKey) ?? ""

        if let queueBaseRoot {
            self.queueBaseRoot = queueBaseRoot
        } else {
            let applicationSupport = FileManager.default.urls(
                for: .applicationSupportDirectory,
                in: .userDomainMask
            ).first ?? FileManager.default.temporaryDirectory
            self.queueBaseRoot = applicationSupport
                .appending(path: "CargoDispatchDriver", directoryHint: .isDirectory)
                .appending(path: "outbound-queue", directoryHint: .isDirectory)
        }

        if !driverURL.isEmpty {
            Task { [weak self] in
                await Task.yield()
                self?.connect()
            }
        }
    }

    func connect() {
        do {
            let target = try SessionLink.parseDriverURL(driverURL)
            let client = ControlAPI(baseURL: target.baseURL, token: target.token)
            let queue = PendingOutboundQueue.forSession(
                baseRoot: queueBaseRoot,
                baseURL: target.baseURL,
                token: target.token
            )
            api = client
            pendingQueue = queue
            busy = true
            errorKey = nil

            Task {
                do {
                    _ = try await client.trip()
                    await flushPending(client: client, queue: queue)
                    trip = try await client.trip()
                    pendingOutbound = await queue.pendingCount()
                    defaults.set(driverURL, forKey: savedDriverURLKey)
                } catch {
                    pendingOutbound = await queue.pendingCount()
                    errorKey = "error.connection"
                }
                busy = false
            }
        } catch {
            errorKey = "error.invalid_driver_url"
        }
    }

    func disconnect() {
        api = nil
        pendingQueue = nil
        trip = nil
        pendingOutbound = 0
        defaults.removeObject(forKey: savedDriverURLKey)
    }

    func refresh() {
        guard let api, let pendingQueue else { return }
        busy = true
        errorKey = nil
        Task {
            await flushPending(client: api, queue: pendingQueue)
            do {
                trip = try await api.trip()
            } catch {
                errorKey = "error.connection"
            }
            pendingOutbound = await pendingQueue.pendingCount()
            busy = false
        }
    }

    func retryPending() {
        guard let api, let pendingQueue else { return }
        Task {
            await flushPending(client: api, queue: pendingQueue)
            if let refreshed = try? await api.trip() {
                trip = refreshed
            }
            pendingOutbound = await pendingQueue.pendingCount()
        }
    }

    func complete(pointId: String) {
        guard let api else { return }
        runTrip {
            try await api.complete(pointId: pointId)
        }
    }

    func moveOpen(pointId: String, delta: Int) {
        guard let api, let trip else { return }
        guard let ids = RouteOrderPlanner.movedOpenPointIds(
            points: trip.points,
            pointId: pointId,
            delta: delta
        ) else {
            return
        }
        runTrip {
            try await api.reorder(
                basedOnRevision: trip.localRouteRevision,
                openPointIds: ids
            )
        }
    }

    func sendLocation(_ update: LocationUpdate, showBusy: Bool = true) {
        guard let api, let pendingQueue else { return }
        if showBusy {
            busy = true
        }
        errorKey = nil
        Task {
            do {
                try await pendingQueue.enqueueLocation(update)
                let accepted = try await api.sendLocation(update)
                try await pendingQueue.removeLocation(through: accepted.observedAtUnixMs)
                if let current = trip {
                    trip = DriverTrip(
                        tripId: current.tripId,
                        tripVersion: current.tripVersion,
                        localRouteRevision: current.localRouteRevision,
                        points: current.points,
                        roadGeometry: current.roadGeometry,
                        documents: current.documents,
                        latestLocation: accepted
                    )
                }
            } catch is URLError {
                errorKey = nil
            } catch let apiError as ControlAPIError {
                switch apiError {
                case let .http(status, _) where status == 409:
                    try? await pendingQueue.removeLocation(through: update.observedAtUnixMs)
                    if let refreshed = try? await api.trip() {
                        trip = refreshed
                    }
                    errorKey = nil
                default:
                    errorKey = "error.location"
                }
            } catch {
                errorKey = "error.location"
            }
            pendingOutbound = await pendingQueue.pendingCount()
            if showBusy {
                busy = false
            }
        }
    }

    func upload(
        data: Data,
        name: String,
        mimeType: String,
        pointId: String? = nil
    ) {
        guard let api, let pendingQueue else { return }

        let documentId = ControlAPI.newDocumentId()
        let createdAtUnixMs = UInt64(max(0, Date().timeIntervalSince1970 * 1000))

        busy = true
        errorKey = nil
        Task {
            do {
                try await pendingQueue.enqueueMedia(
                    documentId: documentId,
                    originalName: name,
                    mimeType: mimeType,
                    data: data,
                    pointId: pointId,
                    createdAtUnixMs: createdAtUnixMs
                )

                trip = try await api.uploadDocument(
                    documentId: documentId,
                    originalName: name,
                    mimeType: mimeType,
                    data: data,
                    pointId: pointId,
                    createdAtUnixMs: createdAtUnixMs
                )
                try await pendingQueue.removeMedia(documentId: documentId)
            } catch is URLError {
                errorKey = nil
            } catch {
                errorKey = "error.file"
            }
            pendingOutbound = await pendingQueue.pendingCount()
            busy = false
        }
    }

    func download(
        document: DriverDocument,
        completion: @escaping (Result<Data, Error>) -> Void
    ) {
        guard let api else { return }
        busy = true
        errorKey = nil
        Task {
            do {
                completion(.success(try await api.downloadDocument(documentId: document.documentId)))
            } catch {
                errorKey = "error.connection"
                completion(.failure(error))
            }
            busy = false
        }
    }

    func reportError(_ key: String) {
        errorKey = key
    }

    private func flushPending(
        client: ControlAPI,
        queue: PendingOutboundQueue
    ) async {
        if let update = await queue.pendingLocation() {
            do {
                let accepted = try await client.sendLocation(update)
                try await queue.removeLocation(through: accepted.observedAtUnixMs)
            } catch is URLError {
                return
            } catch let apiError as ControlAPIError {
                if case let .http(status, _) = apiError, status == 409 {
                    try? await queue.removeLocation(through: update.observedAtUnixMs)
                }
            } catch {
                return
            }
        }

        for pending in await queue.pendingMedia() {
            let item = pending.metadata
            do {
                _ = try await client.uploadDocument(
                    documentId: item.documentId,
                    originalName: item.originalName,
                    mimeType: item.mimeType,
                    data: pending.data,
                    pointId: item.pointId,
                    createdAtUnixMs: item.createdAtUnixMs
                )
                try await queue.removeMedia(documentId: item.documentId)
            } catch is URLError {
                return
            } catch is ControlAPIError {
                // Keep permanent server conflicts visible in the queue for manual attention.
                continue
            } catch {
                return
            }
        }
    }

    private func runTrip(_ operation: @escaping () async throws -> DriverTrip) {
        busy = true
        errorKey = nil
        Task {
            do {
                trip = try await operation()
            } catch {
                errorKey = "error.connection"
            }
            busy = false
        }
    }
}
