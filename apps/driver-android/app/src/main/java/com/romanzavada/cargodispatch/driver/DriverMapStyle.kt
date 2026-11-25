package com.romanzavada.cargodispatch.driver

import com.romanzavada.cargodispatch.driver.model.DriverTrip
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

data class DriverMapCoordinate(
    val latitude: Double,
    val longitude: Double,
)

object DriverMapStyle {
    const val OSM_TILE_URL = "https://tile.openstreetmap.org/{z}/{x}/{y}.png"
    const val OSM_ATTRIBUTION = "© OpenStreetMap contributors"

    fun styleJson(trip: DriverTrip): String {
        val roadFeature = trip.roadGeometry
            ?.points
            ?.takeIf { it.size >= 2 }
            ?.let { points ->
                JsonObject(
                    mapOf(
                        "type" to JsonPrimitive("Feature"),
                        "properties" to JsonObject(emptyMap()),
                        "geometry" to JsonObject(
                            mapOf(
                                "type" to JsonPrimitive("LineString"),
                                "coordinates" to JsonArray(
                                    points.map { point ->
                                        JsonArray(
                                            listOf(
                                                JsonPrimitive(point.longitude),
                                                JsonPrimitive(point.latitude),
                                            ),
                                        )
                                    },
                                ),
                            ),
                        ),
                    ),
                )
            }

        val stopFeatures = trip.points.mapNotNull { point ->
            val navigation = point.navigation ?: return@mapNotNull null
            JsonObject(
                mapOf(
                    "type" to JsonPrimitive("Feature"),
                    "properties" to JsonObject(
                        mapOf(
                            "sequence" to JsonPrimitive(point.sequence),
                            "completed" to JsonPrimitive(point.completed),
                        ),
                    ),
                    "geometry" to pointGeometry(navigation.latitude, navigation.longitude),
                ),
            )
        }

        val driverFeatures = trip.latestLocation?.let { location ->
            listOf(
                JsonObject(
                    mapOf(
                        "type" to JsonPrimitive("Feature"),
                        "properties" to JsonObject(emptyMap()),
                        "geometry" to pointGeometry(location.latitude, location.longitude),
                    ),
                ),
            )
        }.orEmpty()

        val sources = linkedMapOf<String, JsonElement>(
            "osm" to JsonObject(
                mapOf(
                    "type" to JsonPrimitive("raster"),
                    "tiles" to JsonArray(listOf(JsonPrimitive(OSM_TILE_URL))),
                    "tileSize" to JsonPrimitive(256),
                    "attribution" to JsonPrimitive(OSM_ATTRIBUTION),
                ),
            ),
            "road" to geoJsonSource(roadFeature?.let(::listOf).orEmpty()),
            "stops" to geoJsonSource(stopFeatures),
            "driver" to geoJsonSource(driverFeatures),
        )

        val layers = JsonArray(
            listOf(
                JsonObject(
                    mapOf(
                        "id" to JsonPrimitive("osm"),
                        "type" to JsonPrimitive("raster"),
                        "source" to JsonPrimitive("osm"),
                    ),
                ),
                JsonObject(
                    mapOf(
                        "id" to JsonPrimitive("road"),
                        "type" to JsonPrimitive("line"),
                        "source" to JsonPrimitive("road"),
                        "paint" to JsonObject(
                            mapOf(
                                "line-color" to JsonPrimitive("#1769aa"),
                                "line-width" to JsonPrimitive(5),
                                "line-opacity" to JsonPrimitive(0.9),
                            ),
                        ),
                    ),
                ),
                JsonObject(
                    mapOf(
                        "id" to JsonPrimitive("stops"),
                        "type" to JsonPrimitive("circle"),
                        "source" to JsonPrimitive("stops"),
                        "paint" to JsonObject(
                            mapOf(
                                "circle-radius" to JsonPrimitive(7),
                                "circle-color" to JsonArray(
                                    listOf(
                                        JsonPrimitive("case"),
                                        JsonArray(
                                            listOf(
                                                JsonPrimitive("boolean"),
                                                JsonArray(listOf(JsonPrimitive("get"), JsonPrimitive("completed"))),
                                                JsonPrimitive(false),
                                            ),
                                        ),
                                        JsonPrimitive("#68727d"),
                                        JsonPrimitive("#1769aa"),
                                    ),
                                ),
                                "circle-stroke-color" to JsonPrimitive("#ffffff"),
                                "circle-stroke-width" to JsonPrimitive(2),
                            ),
                        ),
                    ),
                ),
                JsonObject(
                    mapOf(
                        "id" to JsonPrimitive("driver"),
                        "type" to JsonPrimitive("circle"),
                        "source" to JsonPrimitive("driver"),
                        "paint" to JsonObject(
                            mapOf(
                                "circle-radius" to JsonPrimitive(8),
                                "circle-color" to JsonPrimitive("#d63b2f"),
                                "circle-stroke-color" to JsonPrimitive("#ffffff"),
                                "circle-stroke-width" to JsonPrimitive(3),
                            ),
                        ),
                    ),
                ),
            ),
        )

        return JsonObject(
            mapOf(
                "version" to JsonPrimitive(8),
                "name" to JsonPrimitive("Cargo Dispatch Driver"),
                "sources" to JsonObject(sources),
                "layers" to layers,
            ),
        ).toString()
    }

    fun extent(trip: DriverTrip): List<DriverMapCoordinate> {
        val values = mutableListOf<DriverMapCoordinate>()
        trip.points.mapNotNullTo(values) { point ->
            point.navigation?.let { DriverMapCoordinate(it.latitude, it.longitude) }
        }
        trip.roadGeometry?.points?.mapTo(values) {
            DriverMapCoordinate(it.latitude, it.longitude)
        }
        trip.latestLocation?.let {
            values += DriverMapCoordinate(it.latitude, it.longitude)
        }
        return values
    }

    private fun geoJsonSource(features: List<JsonObject>): JsonObject =
        JsonObject(
            mapOf(
                "type" to JsonPrimitive("geojson"),
                "data" to JsonObject(
                    mapOf(
                        "type" to JsonPrimitive("FeatureCollection"),
                        "features" to JsonArray(features),
                    ),
                ),
            ),
        )

    private fun pointGeometry(latitude: Double, longitude: Double): JsonObject =
        JsonObject(
            mapOf(
                "type" to JsonPrimitive("Point"),
                "coordinates" to JsonArray(
                    listOf(JsonPrimitive(longitude), JsonPrimitive(latitude)),
                ),
            ),
        )
}
