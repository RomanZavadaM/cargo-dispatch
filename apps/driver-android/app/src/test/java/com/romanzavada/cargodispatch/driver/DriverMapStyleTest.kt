package com.romanzavada.cargodispatch.driver

import com.romanzavada.cargodispatch.driver.model.DriverTrip
import kotlinx.serialization.json.Json
import org.junit.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class DriverMapStyleTest {
    private val json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
    }

    private fun trip(): DriverTrip =
        json.decodeFromString(
            """
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
                },
                {
                  "point_id": "52525252525252525252525252525252",
                  "sequence": 2,
                  "title": "Customer",
                  "completed": true,
                  "navigation": {
                    "latitude": 49.8430,
                    "longitude": 24.0360,
                    "address": "Synthetic customer"
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
                  {"latitude": 49.8410, "longitude": 24.0330},
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
            """.trimIndent(),
        )

    @Test
    fun style_uses_osm_attribution_current_geometry_stops_and_driver_location() {
        val style = DriverMapStyle.styleJson(trip())

        assertTrue(style.contains(DriverMapStyle.OSM_TILE_URL))
        assertTrue(style.contains(DriverMapStyle.OSM_ATTRIBUTION))
        assertTrue(style.contains("LineString"))
        assertTrue(style.contains("49.841"))
        assertTrue(style.contains("24.0311"))
        assertTrue(style.contains("\"completed\":true"))
    }

    @Test
    fun extent_contains_route_geometry_navigation_and_latest_location() {
        val values = DriverMapStyle.extent(trip())

        assertEquals(6, values.size)
        assertTrue(values.any { it.latitude == 49.8401 && it.longitude == 24.0311 })
    }

    @Test
    fun missing_geometry_does_not_invent_road_line() {
        val withoutGeometry = trip().copy(roadGeometry = null)
        val style = DriverMapStyle.styleJson(withoutGeometry)

        assertFalse(style.contains("\"type\":\"LineString\""))
    }
}
