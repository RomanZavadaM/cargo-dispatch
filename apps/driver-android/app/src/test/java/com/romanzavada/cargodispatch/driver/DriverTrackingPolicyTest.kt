package com.romanzavada.cargodispatch.driver

import java.nio.file.Files
import kotlin.test.assertEquals
import kotlin.test.assertNotSame
import kotlin.test.assertSame
import org.junit.Test

class DriverTrackingPolicyTest {
    @Test
    fun background_tracking_uses_deliberate_interval_and_distance_filter() {
        assertEquals(15_000L, DriverTrackingPolicy.MIN_TIME_MS)
        assertEquals(25f, DriverTrackingPolicy.MIN_DISTANCE_M)
    }

    @Test
    fun activity_and_tracking_service_share_one_session_queue_instance() {
        val root = Files.createTempDirectory("cargo-driver-shared-queue").toFile()

        val first = PendingOutboundQueue.forSession(
            root,
            "http://127.0.0.1:8765",
            "0123456789abcdef0123456789abcdef",
        )
        val second = PendingOutboundQueue.forSession(
            root,
            "http://127.0.0.1:8765",
            "0123456789abcdef0123456789abcdef",
        )
        val otherSession = PendingOutboundQueue.forSession(
            root,
            "http://127.0.0.1:8765",
            "fedcba9876543210fedcba9876543210",
        )

        assertSame(first, second)
        assertNotSame(first, otherSession)
    }
}
