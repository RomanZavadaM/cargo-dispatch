package com.romanzavada.cargodispatch.driver.core

/** What went wrong, independent of the interface language. */
enum class DriverProblem {
    LinkInvalid,
    LinkScheme,
    LinkShape,
    NoConnection,
    LinkExpired,
    RouteChanged,
    FileTooLarge,
    Rejected,
    FileUnreadable,
    FileNotSaved,
    LocationPermission,
    LocationUnavailable,
    Unknown,
    ;

    companion object {
        fun fromStatus(status: Int): DriverProblem = when (status) {
            401, 403, 404 -> LinkExpired
            409 -> RouteChanged
            413 -> FileTooLarge
            in 400..499 -> Rejected
            else -> Unknown
        }

        /**
         * A refusal that repeating the same request cannot fix. Missing or expired
         * links, timeouts and rate limits are retried later.
         */
        fun isPermanentRejection(status: Int): Boolean =
            status in 400..499 && status !in setOf(401, 403, 404, 408, 429)
    }
}
