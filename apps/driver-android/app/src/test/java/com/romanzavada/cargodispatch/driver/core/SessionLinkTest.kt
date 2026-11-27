package com.romanzavada.cargodispatch.driver.core

import org.junit.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class SessionLinkTest {
    @Test
    fun emulator_rewrites_host_loopback_but_keeps_port_and_token() {
        val result = SessionLink.parseDriverUrl(
            "http://127.0.0.1:43821/driver/trip-1234567890abcdef",
            emulatorMode = true,
        )
        assertEquals("http://10.0.2.2:43821", result.baseUrl)
        assertEquals("trip-1234567890abcdef", result.token)
    }

    @Test
    fun physical_device_keeps_lan_host() {
        val result = SessionLink.parseDriverUrl(
            "http://192.0.2.10:43821/driver/trip-1234567890abcdef",
            emulatorMode = false,
        )
        assertEquals("http://192.0.2.10:43821", result.baseUrl)
    }

    @Test
    fun arbitrary_url_is_rejected() {
        assertFailsWith<IllegalArgumentException> {
            SessionLink.parseDriverUrl("https://example.test/not-a-driver-url", true)
        }
    }
}
