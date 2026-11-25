import Combine
import Foundation

@MainActor
final class DriverStore: ObservableObject {
    @Published var driverURL = ""
    @Published var trip: DriverTrip?
    @Published var busy = false
    @Published var errorKey: String?

    private var api: ControlAPI?

    func connect() {
        do {
            let target = try SessionLink.parseDriverURL(driverURL)
            api = ControlAPI(baseURL: target.baseURL, token: target.token)
            refresh()
        } catch {
            errorKey = "error.invalid_driver_url"
        }
    }

    func refresh() {
        guard let api else { return }
        run {
            try await api.trip()
        }
    }

    func complete(pointId: String) {
        guard let api else { return }
        run {
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
        run {
            try await api.reorder(
                basedOnRevision: trip.localRouteRevision,
                openPointIds: ids
            )
        }
    }

    private func run(_ operation: @escaping @Sendable () async throws -> DriverTrip) {
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
