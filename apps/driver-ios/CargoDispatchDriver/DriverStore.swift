import Combine
import Foundation

@MainActor
final class DriverStore: ObservableObject {
    @Published var driverURL: String
    @Published var trip: DriverTrip?
    @Published var busy = false
    @Published var errorKey: String?

    private var api: ControlAPI?
    private let defaults: UserDefaults
    private let savedDriverURLKey = "cargo_dispatch_driver_url"

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        self.driverURL = defaults.string(forKey: savedDriverURLKey) ?? ""

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
            api = client
            busy = true
            errorKey = nil
            Task {
                do {
                    trip = try await client.trip()
                    defaults.set(driverURL, forKey: savedDriverURLKey)
                } catch {
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
        trip = nil
        defaults.removeObject(forKey: savedDriverURLKey)
    }

    func refresh() {
        guard let api else { return }
        runTrip {
            try await api.trip()
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

    func sendLocation(_ update: LocationUpdate) {
        guard let api else { return }
        busy = true
        errorKey = nil
        Task {
            do {
                let accepted = try await api.sendLocation(update)
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
            } catch {
                errorKey = "error.location"
            }
            busy = false
        }
    }

    func upload(data: Data, name: String, mimeType: String) {
        guard let api else { return }
        runTrip {
            try await api.uploadDocument(
                originalName: name,
                mimeType: mimeType,
                data: data
            )
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
