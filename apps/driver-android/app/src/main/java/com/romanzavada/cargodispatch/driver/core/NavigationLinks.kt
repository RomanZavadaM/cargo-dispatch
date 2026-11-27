package com.romanzavada.cargodispatch.driver.core

import java.net.URLEncoder
import java.util.Locale

/** Links that hand a destination over to a navigation application. */
object NavigationLinks {
    /** `geo:` lets the driver choose any installed navigator. */
    fun geo(latitude: Double, longitude: Double, title: String): String {
        val position = position(latitude, longitude)
        val label = URLEncoder.encode(title, "UTF-8").replace("+", "%20")
        return "geo:$position?q=$position($label)"
    }

    /** Browser fallback, the same destination link the driver page uses. */
    fun web(latitude: Double, longitude: Double): String =
        "https://www.google.com/maps/dir/?api=1&destination=" +
            position(latitude, longitude).replace(",", "%2C")

    private fun position(latitude: Double, longitude: Double): String =
        String.format(Locale.ROOT, "%.7f,%.7f", latitude, longitude)
}
