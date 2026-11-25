package com.romanzavada.cargodispatch.driver.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class DriverNavigation(
    val latitude: Double,
    val longitude: Double,
    val address: String? = null,
)

@Serializable
data class DriverPoint(
    @SerialName("point_id")
    val pointId: String,
    val sequence: Int,
    val title: String,
    val completed: Boolean,
    val navigation: DriverNavigation? = null,
)

@Serializable
data class DriverLocation(
    val latitude: Double,
    val longitude: Double,
    @SerialName("accuracy_m")
    val accuracyM: Double? = null,
    @SerialName("observed_at_unix_ms")
    val observedAtUnixMs: Long,
)

@Serializable
data class DriverDocument(
    @SerialName("document_id")
    val documentId: String,
    val direction: String,
    @SerialName("original_name")
    val originalName: String,
    @SerialName("mime_type")
    val mimeType: String,
    val size: Long,
    @SerialName("point_id")
    val pointId: String? = null,
)

@Serializable
data class DriverRoadGeometryPoint(
    val latitude: Double,
    val longitude: Double,
)

@Serializable
data class DriverRoutingIdentity(
    val engine: String,
    @SerialName("engine_version")
    val engineVersion: String,
    @SerialName("map_dataset_id")
    val mapDatasetId: String,
    @SerialName("profile_fingerprint")
    val profileFingerprint: String,
)

@Serializable
data class DriverRoadGeometry(
    val routing: DriverRoutingIdentity,
    val points: List<DriverRoadGeometryPoint>,
)

@Serializable
data class DriverTrip(
    @SerialName("trip_id")
    val tripId: String,
    @SerialName("trip_version")
    val tripVersion: Long,
    @SerialName("local_route_revision")
    val localRouteRevision: Long,
    val points: List<DriverPoint>,
    @SerialName("road_geometry")
    val roadGeometry: DriverRoadGeometry? = null,
    val documents: List<DriverDocument> = emptyList(),
    @SerialName("latest_location")
    val latestLocation: DriverLocation? = null,
)

@Serializable
data class RouteOrderRequest(
    @SerialName("based_on_revision")
    val basedOnRevision: Long,
    @SerialName("open_point_ids")
    val openPointIds: List<String>,
)


@Serializable
data class LocationUpdate(
    val latitude: Double,
    val longitude: Double,
    @SerialName("accuracy_m")
    val accuracyM: Double? = null,
    @SerialName("observed_at_unix_ms")
    val observedAtUnixMs: Long,
)
