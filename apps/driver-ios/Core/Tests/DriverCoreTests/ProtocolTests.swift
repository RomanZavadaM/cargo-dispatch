import Foundation
import Testing
@testable import DriverCore

@Test
func parsesTripSnapshotWithRoadGeometry() throws {
    let json = """
    {
      "trip_id":"41414141414141414141414141414141",
      "trip_version":1,
      "local_route_revision":0,
      "points":[{
        "point_id":"51515151515151515151515151515151",
        "sequence":1,
        "title":"Depot",
        "completed":false,
        "navigation":{"latitude":49.8393,"longitude":24.0296,"address":"Synthetic depot"}
      }],
      "road_geometry":{
        "routing":{
          "engine":"valhalla",
          "engine_version":"3.9.2",
          "map_dataset_id":"fixture",
          "profile_fingerprint":"abc"
        },
        "points":[
          {"latitude":49.8393,"longitude":24.0296},
          {"latitude":49.8430,"longitude":24.0360}
        ]
      },
      "documents":[],
      "latest_location":null
    }
    """

    let trip = try JSONDecoder().decode(DriverTrip.self, from: Data(json.utf8))
    #expect(trip.points.count == 1)
    #expect(trip.points[0].navigation?.address == "Synthetic depot")
    #expect(trip.roadGeometry?.points.count == 2)
}
