package com.romanzavada.cargodispatch.driver.net

import com.romanzavada.cargodispatch.driver.FileTransferApi
import com.romanzavada.cargodispatch.driver.PendingMediaMetadata
import com.romanzavada.cargodispatch.driver.model.DriverLocation
import com.romanzavada.cargodispatch.driver.model.DriverTrip
import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import com.romanzavada.cargodispatch.driver.model.RouteOrderRequest
import com.romanzavada.cargodispatch.driver.model.UploadStatus
import java.io.File
import java.io.IOException
import java.io.RandomAccessFile
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
) : FileTransferApi {
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

    override suspend fun uploadStatus(documentId: String): UploadStatus =
        requestJson("GET", "/api/trip/$token/uploads/$documentId")

    override suspend fun uploadPart(
        entry: PendingMediaMetadata,
        offset: Long,
        total: Long,
        part: ByteArray,
        sha256: String,
    ): UploadStatus =
        requestJson(
            method = "POST",
            path = "/api/trip/$token/uploads/${entry.documentId}" +
                "?offset=$offset&total=$total&part_sha256=$sha256",
            rawBody = part,
            headers = uploadHeaders(entry),
        )

    override suspend fun uploadWhole(entry: PendingMediaMetadata, file: File): Unit =
        withContext(Dispatchers.IO) {
            val connection = open("POST", "/api/trip/$token/documents/${entry.documentId}")
            try {
                uploadHeaders(entry).forEach { (name, value) ->
                    connection.setRequestProperty(name, value)
                }
                connection.doOutput = true
                connection.setFixedLengthStreamingMode(file.length())
                connection.outputStream.use { output ->
                    file.inputStream().use { it.copyTo(output) }
                }
                requireSuccess(connection)
                connection.inputStream.use { it.readBytes() }
                Unit
            } finally {
                connection.disconnect()
            }
        }

    override suspend fun downloadPart(
        documentId: String,
        offset: Long,
        length: Int,
        destination: File,
    ): Boolean = withContext(Dispatchers.IO) {
        val connection = open(
            "GET",
            "/api/trip/$token/documents/$documentId?offset=$offset&length=$length",
        )
        try {
            val status = requireSuccess(connection)
            if (status == HttpURLConnection.HTTP_PARTIAL) {
                val part = connection.inputStream.use { it.readBytes() }
                // A part cut short by the network is not appended.
                if (part.size != length) throw IOException("Incomplete part")
                RandomAccessFile(destination, "rw").use { target ->
                    target.setLength(offset)
                    target.seek(offset)
                    target.write(part)
                }
                true
            } else {
                // Control 0.3.0 and earlier sends the whole file.
                destination.outputStream().use { output ->
                    connection.inputStream.use { it.copyTo(output) }
                }
                false
            }
        } finally {
            connection.disconnect()
        }
    }

    private fun uploadHeaders(entry: PendingMediaMetadata): Map<String, String> =
        buildMap {
            put("Content-Type", entry.mimeType.ifBlank { "application/octet-stream" })
            put("X-File-Name", percentEncodeHeader(entry.originalName))
            put("X-Created-At-Unix-Ms", entry.createdAtUnixMs.toString())
            entry.pointId?.let { put("X-Point-Id", it) }
        }

    private fun open(method: String, path: String): HttpURLConnection =
        (URL(baseUrl + path).openConnection() as HttpURLConnection).apply {
            requestMethod = method
            connectTimeout = 10_000
            readTimeout = 30_000
            useCaches = false
            setRequestProperty("Accept", "*/*")
        }

    private fun requireSuccess(connection: HttpURLConnection): Int {
        val status = connection.responseCode
        if (status !in 200..299) {
            val answer = connection.errorStream?.use { it.readBytes() } ?: ByteArray(0)
            throw ControlApiException(
                status,
                answer.toString(Charsets.UTF_8).ifBlank { "Control request failed" },
            )
        }
        return status
    }

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
