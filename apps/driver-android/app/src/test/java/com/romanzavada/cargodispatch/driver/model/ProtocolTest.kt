package com.romanzavada.cargodispatch.driver.model

import org.junit.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlinx.serialization.json.Json

class ProtocolTest {
    private val json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
    }

    @Test
    fun parses_control_trip_snapshot_with_geometry_and_documents() {
        val payload = """
            {
              "trip_id": "41414141414141414141414141414141",
              "trip_version": 1,
              "local_route_revision": 0,
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
              "documents": [
                {
                  "document_id": "61616161616161616161616161616161",
                  "direction": "from_operator",
                  "original_name": "ttn.pdf",
                  "mime_type": "application/pdf",
                  "size": 1024,
                  "point_id": null
                }
              ],
              "latest_location": null
            }
        """.trimIndent()

        val trip = json.decodeFromString<DriverTrip>(payload)
        assertEquals(1, trip.points.size)
        assertEquals("Synthetic depot", trip.points.single().navigation?.address)
        assertEquals("ttn.pdf", trip.documents.single().originalName)
        assertNotNull(trip.roadGeometry)
        assertEquals(2, trip.roadGeometry?.points?.size)
    }
}
