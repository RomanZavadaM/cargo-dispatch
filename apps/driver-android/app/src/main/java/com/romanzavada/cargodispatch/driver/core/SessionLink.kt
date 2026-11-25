package com.romanzavada.cargodispatch.driver.core

import java.net.URI

data class SessionTarget(
    val baseUrl: String,
    val token: String,
)

object SessionLink {
    private val tokenPattern = Regex("[A-Za-z0-9_-]{16,128}")

    fun parseDriverUrl(raw: String, emulatorMode: Boolean): SessionTarget {
        val uri = runCatching { URI(raw.trim()) }
            .getOrElse { throw IllegalArgumentException("Некоректний Driver URL") }

        require(uri.scheme == "http" || uri.scheme == "https") {
            "Driver URL має починатися з http:// або https://"
        }
        val host = uri.host ?: throw IllegalArgumentException("Driver URL не має host")
        val segments = uri.path.orEmpty().trim('/').split('/')
        require(segments.size == 2 && segments[0] == "driver") {
            "Очікується Driver URL виду /driver/<token>"
        }
        val token = segments[1]
        require(tokenPattern.matches(token)) {
            "Некоректний session token"
        }

        val effectiveHost = if (emulatorMode && (host == "127.0.0.1" || host == "localhost")) {
            "10.0.2.2"
        } else {
            host
        }
        val authority = if (uri.port >= 0) "${effectiveHost}:${uri.port}" else effectiveHost
        return SessionTarget(
            baseUrl = "${uri.scheme}://${authority}",
            token = token,
        )
    }
}
