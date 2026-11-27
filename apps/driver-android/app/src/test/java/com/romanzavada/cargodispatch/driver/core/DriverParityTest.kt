package com.romanzavada.cargodispatch.driver.core

import java.util.Locale
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import org.junit.Test

class DriverParityTest {
    @Test
    fun navigation_links_use_decimal_points_and_encode_the_title() {
        val previous = Locale.getDefault()
        Locale.setDefault(Locale.forLanguageTag("uk-UA"))
        try {
            assertEquals(
                "geo:49.8420000,24.0520000?q=49.8420000,24.0520000(%D0%A1%D0%BA%D0%BB%D0%B0%D0%B4%20%231)",
                NavigationLinks.geo(49.842, 24.052, "Склад #1"),
            )
            assertEquals(
                "https://www.google.com/maps/dir/?api=1&destination=49.8420000%2C24.0520000",
                NavigationLinks.web(49.842, 24.052),
            )
        } finally {
            Locale.setDefault(previous)
        }
    }

    @Test
    fun file_sizes_are_readable() {
        assertEquals(FileSize("900", SizeUnit.Bytes), FileSize.of(900, Locale.ROOT))
        assertEquals(FileSize("1.5", SizeUnit.Kilobytes), FileSize.of(1536, Locale.ROOT))
        assertEquals(FileSize("2.0", SizeUnit.Megabytes), FileSize.of(2L * 1024 * 1024, Locale.ROOT))
    }

    @Test
    fun only_refusals_that_repeating_cannot_fix_are_permanent() {
        listOf(400, 409, 413, 415).forEach { assertTrue(DriverProblem.isPermanentRejection(it), "$it") }
        listOf(401, 403, 404, 408, 429, 500, 503).forEach {
            assertFalse(DriverProblem.isPermanentRejection(it), "$it")
        }
        assertEquals(DriverProblem.LinkExpired, DriverProblem.fromStatus(404))
        assertEquals(DriverProblem.FileTooLarge, DriverProblem.fromStatus(413))
        assertEquals(DriverProblem.Unknown, DriverProblem.fromStatus(500))
    }

    @Test
    fun link_problems_are_named_without_interface_text() {
        val shape = assertFailsWith<SessionLinkException> {
            SessionLink.parseDriverUrl("https://example.test/not-a-driver-url", false)
        }
        assertEquals(DriverProblem.LinkShape, shape.problem)
        val scheme = assertFailsWith<SessionLinkException> {
            SessionLink.parseDriverUrl("ftp://example.test/driver/trip-1234567890abcdef", false)
        }
        assertEquals(DriverProblem.LinkScheme, scheme.problem)
        assertTrue(SessionLink.isLoopback("http://127.0.0.1:1/driver/x"))
        assertFalse(SessionLink.isLoopback("http://192.168.1.44:1/driver/x"))
    }
}
