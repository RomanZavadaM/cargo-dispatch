package com.romanzavada.cargodispatch.driver.core

import java.net.URI

data class SessionTarget(
    val baseUrl: String,
    val token: String,
)

class SessionLinkException(val problem: DriverProblem) : IllegalArgumentException(problem.name)

object SessionLink {
    private val tokenPattern = Regex("[A-Za-z0-9_-]{16,128}")
    private val loopbackHosts = setOf("127.0.0.1", "localhost")

    /** True when the link points at the device itself; only an emulator can use it. */
    fun isLoopback(raw: String): Boolean =
        runCatching { URI(raw.trim()).host }.getOrNull() in loopbackHosts

    fun parseDriverUrl(raw: String, emulatorMode: Boolean): SessionTarget {
        val uri = runCatching { URI(raw.trim()) }
            .getOrElse { throw SessionLinkException(DriverProblem.LinkInvalid) }

        if (uri.scheme != "http" && uri.scheme != "https") {
            throw SessionLinkException(DriverProblem.LinkScheme)
        }
        val host = uri.host ?: throw SessionLinkException(DriverProblem.LinkInvalid)
        val segments = uri.path.orEmpty().trim('/').split('/')
        if (segments.size != 2 || segments[0] != "driver") {
            throw SessionLinkException(DriverProblem.LinkShape)
        }
        val token = segments[1]
        if (!tokenPattern.matches(token)) {
            throw SessionLinkException(DriverProblem.LinkShape)
        }

        val effectiveHost = if (emulatorMode && host in loopbackHosts) "10.0.2.2" else host
        val authority = if (uri.port >= 0) "${effectiveHost}:${uri.port}" else effectiveHost
        return SessionTarget(
            baseUrl = "${uri.scheme}://${authority}",
            token = token,
        )
    }
}
