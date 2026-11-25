import Testing
@testable import DriverCore

@Test
func mapSnapshotUsesOnlyCurrentTripGeometryStopsAndLocation() throws {
    let data = Data(
        """
        {
          "trip_id": "41414141414141414141414141414141",
          "trip_version": 1,
          "local_route_revision": 3,
          "points": [
            {
              "point_id": "51515151515151515151515151515151",
              "sequence": 1,
              "title": "Depot",
              "completed": false,
              "navigation": {
                "latitude": 49.8393,
                "longitude": 24.0296,
                "address": "Synthetic depot"
              }
            }
          ],
          "road_geometry": {
            "routing": {
              "engine": "valhalla",
              "engine_version": "3.9.2",
              "map_dataset_id": "fixture",
              "profile_fingerprint": "abc"
            },
            "points": [
              {"latitude": 49.8393, "longitude": 24.0296},
              {"latitude": 49.8430, "longitude": 24.0360}
            ]
          },
          "documents": [],
          "latest_location": {
            "latitude": 49.8401,
            "longitude": 24.0311,
            "accuracy_m": 8.0,
            "observed_at_unix_ms": 1795610000000
          }
        }
        """.utf8
    )

    let trip = try JSONDecoder().decode(DriverTrip.self, from: data)
    let map = DriverMapContent.snapshot(from: trip)

    #expect(map.stops.count == 1)
    #expect(map.roadLine.count == 2)
    #expect(map.driverLocation == DriverMapCoordinate(latitude: 49.8401, longitude: 24.0311))
    #expect(map.extent.count == 4)
}

@Test
func mapSnapshotDoesNotInventRoadGeometry() throws {
    let data = Data(
        """
        {
          "trip_id": "41414141414141414141414141414141",
          "trip_version": 1,
          "local_route_revision": 4,
          "points": [],
          "road_geometry": null,
          "documents": [],
          "latest_location": null
        }
        """.utf8
    )

    let trip = try JSONDecoder().decode(DriverTrip.self, from: data)
    let map = DriverMapContent.snapshot(from: trip)

    #expect(map.roadLine.isEmpty)
    #expect(map.extent.isEmpty)
}
