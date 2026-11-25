import Foundation

public struct DriverMapCoordinate: Equatable, Sendable {
    public let latitude: Double
    public let longitude: Double

    public init(latitude: Double, longitude: Double) {
        self.latitude = latitude
        self.longitude = longitude
    }
}

public struct DriverMapStop: Equatable, Identifiable, Sendable {
    public let pointId: String
    public let sequence: Int
    public let title: String
    public let completed: Bool
    public let coordinate: DriverMapCoordinate

    public var id: String { pointId }
}

public struct DriverMapSnapshot: Equatable, Sendable {
    public let stops: [DriverMapStop]
    public let roadLine: [DriverMapCoordinate]
    public let driverLocation: DriverMapCoordinate?

    public var extent: [DriverMapCoordinate] {
        stops.map(\.coordinate) + roadLine + (driverLocation.map { [$0] } ?? [])
    }
}

public enum DriverMapContent {
    public static func snapshot(from trip: DriverTrip) -> DriverMapSnapshot {
        let stops = trip.points.compactMap { point -> DriverMapStop? in
            guard let navigation = point.navigation else { return nil }
            return DriverMapStop(
                pointId: point.pointId,
                sequence: point.sequence,
                title: point.title,
                completed: point.completed,
                coordinate: DriverMapCoordinate(
                    latitude: navigation.latitude,
                    longitude: navigation.longitude
                )
            )
        }

        let roadLine = trip.roadGeometry?.points.map {
            DriverMapCoordinate(latitude: $0.latitude, longitude: $0.longitude)
        } ?? []

        let driverLocation = trip.latestLocation.map {
            DriverMapCoordinate(latitude: $0.latitude, longitude: $0.longitude)
        }

        return DriverMapSnapshot(
            stops: stops,
            roadLine: roadLine,
            driverLocation: driverLocation
        )
    }
}
