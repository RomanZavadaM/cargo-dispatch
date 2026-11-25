package com.romanzavada.cargodispatch.driver.net

import com.romanzavada.cargodispatch.driver.model.DriverTrip
import com.romanzavada.cargodispatch.driver.model.RouteOrderRequest
import java.net.HttpURLConnection
import java.net.URL
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
            "Control base URL має бути HTTP(S)"
        }
    }

    suspend fun trip(): DriverTrip =
        request("GET", "/api/trip/$token")

    suspend fun complete(pointId: String): DriverTrip =
        request("POST", "/api/trip/$token/complete/$pointId")

    suspend fun reorder(basedOnRevision: Long, openPointIds: List<String>): DriverTrip =
        request(
            method = "POST",
            path = "/api/trip/$token/route/order",
            body = json.encodeToString(RouteOrderRequest(basedOnRevision, openPointIds)),
        )

    private suspend inline fun <reified T> request(
        method: String,
        path: String,
        body: String? = null,
    ): T = withContext(Dispatchers.IO) {
        val connection = (URL(baseUrl + path).openConnection() as HttpURLConnection).apply {
            requestMethod = method
            connectTimeout = 10_000
            readTimeout = 20_000
            setRequestProperty("Accept", "application/json")
            useCaches = false
            if (body != null) {
                doOutput = true
                setRequestProperty("Content-Type", "application/json")
            }
        }

        try {
            if (body != null) {
                connection.outputStream.bufferedWriter(Charsets.UTF_8).use { it.write(body) }
            }
            val status = connection.responseCode
            val stream = if (status in 200..299) connection.inputStream else connection.errorStream
            val text = stream?.bufferedReader(Charsets.UTF_8)?.use { it.readText() }.orEmpty()
            if (status !in 200..299) {
                throw ControlApiException(status, text.ifBlank { "Control request failed" })
            }
            json.decodeFromString<T>(text)
        } finally {
            connection.disconnect()
        }
    }
}

class ControlApiException(
    val statusCode: Int,
    response: String,
) : RuntimeException("Control HTTP ${statusCode}: ${response}")
