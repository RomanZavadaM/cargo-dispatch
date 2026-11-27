import Combine
import Foundation

@MainActor
final class DriverStore: ObservableObject {
    @Published var driverURL: String
    @Published var trip: DriverTrip?
    @Published var busy = false
    @Published var offline = false
    @Published var pendingOutbound = 0
    @Published var rejected: [PendingMediaMetadata] = []
    @Published var problem: DriverProblem?

    private var api: ControlAPI?
    private var pendingQueue: PendingOutboundQueue?
    private let sender = OutboundSender()
    private let sessionStore: SecureSessionStore
    private let queueBaseRoot: URL

    /// Grows with every action of the driver, so a slower background refresh is dropped.
    private var generation = 0

    init(
        defaults: UserDefaults = .standard,
        queueBaseRoot: URL? = nil
    ) {
        let secureSessionStore = SecureSessionStore(defaults: defaults)
        self.sessionStore = secureSessionStore
        self.driverURL = secureSessionStore.loadDriverURL() ?? ""

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
        let target: SessionTarget
        do {
            target = try SessionLink.parseDriverURL(driverURL)
        } catch SessionLinkError.unsupportedScheme {
            problem = .linkScheme
            return
        } catch SessionLinkError.invalidURL {
            problem = .linkInvalid
            return
        } catch {
            problem = .linkShape
            return
        }

        let client = ControlAPI(baseURL: target.baseURL, token: target.token)
        let queue = PendingOutboundQueue.forSession(
            baseRoot: queueBaseRoot,
            baseURL: target.baseURL,
            token: target.token
        )
        api = client
        pendingQueue = queue

        perform {
            _ = try await client.trip()
            do {
                // The first successful authenticated trip read proves the Driver
                // session is valid. Persist it securely before any queue flushing
                // or follow-up refresh so a fast app stop cannot leave the legacy
                // raw URL on disk.
                try self.sessionStore.save(driverURL: self.driverURL)
            } catch {
                // Keep the live in-memory session, but never fall back to
                // persisting a raw token if Keychain storage fails.
                self.sessionStore.clear()
            }
            await self.flush(client: client, queue: queue)
            self.trip = try await client.trip()
        }
    }

    func disconnect() {
        generation += 1
        api = nil
        pendingQueue = nil
        trip = nil
        offline = false
        problem = nil
        pendingOutbound = 0
        rejected = []
        sessionStore.clear()
    }

    func refresh() {
        guard let api, let pendingQueue else { return }
        perform {
            await self.flush(client: api, queue: pendingQueue)
            self.trip = try await api.trip()
        }
    }

    /// Periodic update while the screen is open. It never blocks the buttons.
    func refreshQuietly() async {
        guard let client = api, let queue = pendingQueue, !busy else { return }
        let startedAt = generation
        await flush(client: client, queue: queue)
        do {
            let fetched = try await client.trip()
            guard generation == startedAt, api === client else { return }
            if trip != fetched {
                trip = fetched
            }
            offline = false
            // A message about a lost connection is no longer true.
            if problem == .noConnection {
                problem = nil
            }
        } catch is URLError {
            if generation == startedAt {
                offline = true
            }
        } catch let error as ControlAPIError {
            if generation == startedAt {
                problem = Self.problem(for: error)
            }
        } catch {
            // An unreadable answer must not disturb the driver; the next round retries.
        }
        await updateQueueState(queue)
    }

    func complete(pointId: String) {
        guard let api else { return }
        perform {
            self.trip = try await api.complete(pointId: pointId)
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
        perform {
            self.trip = try await api.reorder(
                basedOnRevision: trip.localRouteRevision,
                openPointIds: ids
            )
        }
    }

    func sendLocation(_ update: LocationUpdate) {
        guard let api, let pendingQueue else { return }
        perform {
            try await pendingQueue.enqueueLocation(update)
            await self.flush(client: api, queue: pendingQueue)
            self.trip = try await api.trip()
        }
    }

    /// A position from background tracking: recorded and sent without touching the screen.
    func trackLocation(_ update: LocationUpdate) {
        guard let client = api, let queue = pendingQueue else { return }
        Task {
            do {
                try await queue.enqueueLocation(update)
            } catch {
                problem = .unknown
            }
            let accepted = await sender.sendLocations(queue: queue) {
                try await client.sendLocation($0)
            }
            if let accepted, let current = trip, api === client {
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
            await updateQueueState(queue)
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
        let known = trip?.points.contains { $0.pointId == pointId } ?? false

        perform {
            try await pendingQueue.enqueueMedia(
                documentId: documentId,
                originalName: name,
                mimeType: mimeType,
                data: data,
                pointId: known ? pointId : nil,
                createdAtUnixMs: createdAtUnixMs
            )
            await self.flush(client: api, queue: pendingQueue)
            self.trip = try await api.trip()
        }
    }

    func retryRejected(documentId: String) {
        guard let pendingQueue else { return }
        Task {
            try? await pendingQueue.retryRejected(documentId: documentId)
            await updateQueueState(pendingQueue)
            refresh()
        }
    }

    func discardRejected(documentId: String) {
        guard let pendingQueue else { return }
        Task {
            try? await pendingQueue.removeMedia(documentId: documentId)
            await updateQueueState(pendingQueue)
        }
    }

    func download(
        document: DriverDocument,
        completion: @escaping @MainActor (Data) -> Void
    ) {
        guard let api else { return }
        perform {
            completion(try await api.downloadDocument(documentId: document.documentId))
        }
    }

    func report(_ problem: DriverProblem) {
        self.problem = problem
    }

    private static func problem(for error: ControlAPIError) -> DriverProblem {
        if case let .http(status, _) = error {
            return DriverProblem.fromStatus(status)
        }
        return .unknown
    }

    private func flush(client: ControlAPI, queue: PendingOutboundQueue) async {
        await sender.sendLocations(queue: queue) {
            try await client.sendLocation($0)
        }
        await sender.sendMedia(client: client, queue: queue)
    }

    private func updateQueueState(_ queue: PendingOutboundQueue) async {
        let count = await queue.pendingCount()
        let refused = await queue.rejectedMedia()
        if pendingQueue === queue {
            pendingOutbound = count
            rejected = refused
        }
    }

    /// One action of the driver: shows progress and turns failures into a message.
    private func perform(_ operation: @escaping @MainActor () async throws -> Void) {
        generation += 1
        busy = true
        problem = nil
        let queue = pendingQueue
        Task {
            do {
                try await operation()
                offline = false
            } catch is URLError {
                problem = .noConnection
                offline = true
            } catch let error as ControlAPIError {
                problem = Self.problem(for: error)
                if problem == .routeChanged, let api, let refreshed = try? await api.trip() {
                    trip = refreshed
                }
            } catch {
                problem = .unknown
            }
            if let queue {
                await updateQueueState(queue)
            }
            busy = false
        }
    }
}
