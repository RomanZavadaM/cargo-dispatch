import Foundation

public struct DriverNavigation: Codable, Equatable, Sendable {
    public let latitude: Double
    public let longitude: Double
    public let address: String?
}

public struct DriverPoint: Codable, Equatable, Identifiable, Sendable {
    public let pointId: String
    public let sequence: Int
    public let title: String
    public let completed: Bool
    public let navigation: DriverNavigation?

    public var id: String { pointId }

    enum CodingKeys: String, CodingKey {
        case pointId = "point_id"
        case sequence
        case title
        case completed
        case navigation
    }
}

public struct DriverLocation: Codable, Equatable, Sendable {
    public let latitude: Double
    public let longitude: Double
    public let accuracyM: Double?
    public let observedAtUnixMs: UInt64

    enum CodingKeys: String, CodingKey {
        case latitude
        case longitude
        case accuracyM = "accuracy_m"
        case observedAtUnixMs = "observed_at_unix_ms"
    }
}

public struct DriverDocument: Codable, Equatable, Identifiable, Sendable {
    public let documentId: String
    public let direction: String
    public let originalName: String
    public let mimeType: String
    public let size: UInt64
    public let pointId: String?

    public var id: String { documentId }

    enum CodingKeys: String, CodingKey {
        case documentId = "document_id"
        case direction
        case originalName = "original_name"
        case mimeType = "mime_type"
        case size
        case pointId = "point_id"
    }
}

public struct DriverRoadGeometryPoint: Codable, Equatable, Sendable {
    public let latitude: Double
    public let longitude: Double
}

public struct DriverRoutingIdentity: Codable, Equatable, Sendable {
    public let engine: String
    public let engineVersion: String
    public let mapDatasetId: String
    public let profileFingerprint: String

    enum CodingKeys: String, CodingKey {
        case engine
        case engineVersion = "engine_version"
        case mapDatasetId = "map_dataset_id"
        case profileFingerprint = "profile_fingerprint"
    }
}

public struct DriverRoadGeometry: Codable, Equatable, Sendable {
    public let routing: DriverRoutingIdentity
    public let points: [DriverRoadGeometryPoint]
}

public struct DriverTrip: Codable, Equatable, Sendable {
    public let tripId: String
    public let tripVersion: UInt64
    public let localRouteRevision: UInt64
    public let points: [DriverPoint]
    public let roadGeometry: DriverRoadGeometry?
    public let documents: [DriverDocument]
    public let latestLocation: DriverLocation?

    enum CodingKeys: String, CodingKey {
        case tripId = "trip_id"
        case tripVersion = "trip_version"
        case localRouteRevision = "local_route_revision"
        case points
        case roadGeometry = "road_geometry"
        case documents
        case latestLocation = "latest_location"
    }
}

public struct RouteOrderRequest: Codable, Equatable, Sendable {
    public let basedOnRevision: UInt64
    public let openPointIds: [String]

    public init(basedOnRevision: UInt64, openPointIds: [String]) {
        self.basedOnRevision = basedOnRevision
        self.openPointIds = openPointIds
    }

    enum CodingKeys: String, CodingKey {
        case basedOnRevision = "based_on_revision"
        case openPointIds = "open_point_ids"
    }
}
