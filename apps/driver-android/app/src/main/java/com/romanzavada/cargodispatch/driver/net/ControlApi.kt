package com.romanzavada.cargodispatch.driver.net

import com.romanzavada.cargodispatch.driver.model.DriverLocation
import com.romanzavada.cargodispatch.driver.model.DriverTrip
import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import com.romanzavada.cargodispatch.driver.model.RouteOrderRequest
import java.net.HttpURLConnection
import java.net.URL
import java.security.SecureRandom
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

class ControlApi(
    baseUrl: String,
    private val token: String,
) {
    private val baseUrl = baseUrl.trimEnd('/')
    private val json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
    }

    init {
        require(this.baseUrl.startsWith("http://") || this.baseUrl.startsWith("https://")) {
            "Control base URL must be HTTP(S)"
        }
    }

    suspend fun trip(): DriverTrip =
        requestJson("GET", "/api/trip/$token")

    suspend fun complete(pointId: String): DriverTrip =
        requestJson("POST", "/api/trip/$token/complete/$pointId")

    suspend fun reorder(basedOnRevision: Long, openPointIds: List<String>): DriverTrip =
        requestJson(
            method = "POST",
            path = "/api/trip/$token/route/order",
            body = json.encodeToString(RouteOrderRequest(basedOnRevision, openPointIds)),
        )

    suspend fun sendLocation(update: LocationUpdate): DriverLocation =
        requestJson(
            method = "POST",
            path = "/api/trip/$token/location",
            body = json.encodeToString(update),
        )

    suspend fun downloadDocument(documentId: String): ByteArray =
        requestBytes(
            method = "GET",
            path = "/api/trip/$token/documents/$documentId",
        )

    suspend fun uploadDocument(
        documentId: String = newDocumentId(),
        originalName: String,
        mimeType: String,
        bytes: ByteArray,
        pointId: String? = null,
        createdAtUnixMs: Long = System.currentTimeMillis(),
    ): DriverTrip =
        requestJson(
            method = "POST",
            path = "/api/trip/$token/documents/$documentId",
            rawBody = bytes,
            headers = buildMap {
                put("Content-Type", mimeType.ifBlank { "application/octet-stream" })
                put("X-File-Name", percentEncodeHeader(originalName))
                put("X-Created-At-Unix-Ms", createdAtUnixMs.toString())
                pointId?.let { put("X-Point-Id", it) }
            },
        )

    private suspend inline fun <reified T> requestJson(
        method: String,
        path: String,
        body: String? = null,
        rawBody: ByteArray? = null,
        headers: Map<String, String> = emptyMap(),
    ): T {
        val bytes = request(
            method = method,
            path = path,
            body = body?.toByteArray(Charsets.UTF_8) ?: rawBody,
            headers = headers + if (body != null) mapOf("Content-Type" to "application/json") else emptyMap(),
            accept = "application/json",
        )
        return json.decodeFromString(bytes.toString(Charsets.UTF_8))
    }

    private suspend fun requestBytes(
        method: String,
        path: String,
    ): ByteArray =
        request(
            method = method,
            path = path,
            body = null,
            headers = emptyMap(),
            accept = "*/*",
        )

    private suspend fun request(
        method: String,
        path: String,
        body: ByteArray?,
        headers: Map<String, String>,
        accept: String,
    ): ByteArray = withContext(Dispatchers.IO) {
        val connection = (URL(baseUrl + path).openConnection() as HttpURLConnection).apply {
            requestMethod = method
            connectTimeout = 10_000
            readTimeout = 30_000
            useCaches = false
            setRequestProperty("Accept", accept)
            headers.forEach { (name, value) -> setRequestProperty(name, value) }
            if (body != null) {
                doOutput = true
                setFixedLengthStreamingMode(body.size)
            }
        }

        try {
            if (body != null) {
                connection.outputStream.use { it.write(body) }
            }
            val status = connection.responseCode
            val stream = if (status in 200..299) connection.inputStream else connection.errorStream
            val bytes = stream?.use { it.readBytes() } ?: ByteArray(0)
            if (status !in 200..299) {
                throw ControlApiException(
                    status,
                    bytes.toString(Charsets.UTF_8).ifBlank { "Control request failed" },
                )
            }
            bytes
        } finally {
            connection.disconnect()
        }
    }

    companion object {
        private val secureRandom = SecureRandom()

        fun newDocumentId(): String {
            val bytes = ByteArray(16)
            secureRandom.nextBytes(bytes)
            if (bytes.all { it == 0.toByte() }) {
                bytes[0] = 1
            }
            return bytes.joinToString(separator = "") { "%02x".format(it.toInt() and 0xff) }
        }

        fun percentEncodeHeader(value: String): String =
            buildString {
                value.toByteArray(Charsets.UTF_8).forEach { byte ->
                    val unsigned = byte.toInt() and 0xff
                    val safe =
                        unsigned in 'a'.code..'z'.code ||
                            unsigned in 'A'.code..'Z'.code ||
                            unsigned in '0'.code..'9'.code ||
                            unsigned == '-'.code ||
                            unsigned == '.'.code ||
                            unsigned == '_'.code ||
                            unsigned == '~'.code
                    if (safe) {
                        append(unsigned.toChar())
                    } else {
                        append('%')
                        append("%02X".format(unsigned))
                    }
                }
            }
    }
}

class ControlApiException(
    val statusCode: Int,
    response: String,
) : RuntimeException("Control HTTP $statusCode: $response")
