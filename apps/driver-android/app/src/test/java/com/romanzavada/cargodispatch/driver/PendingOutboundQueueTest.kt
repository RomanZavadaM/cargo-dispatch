package com.romanzavada.cargodispatch.driver

import com.romanzavada.cargodispatch.driver.model.DriverLocation
import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import com.romanzavada.cargodispatch.driver.net.ControlApiException
import java.io.File
import java.io.IOException
import java.nio.file.Files
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlinx.coroutines.runBlocking
import org.junit.Test

class PendingOutboundQueueTest {
    private fun position(at: Long) = LocationUpdate(49.84, 24.03, 6.0, at)

    private fun accepted(update: LocationUpdate) = DriverLocation(
        update.latitude,
        update.longitude,
        update.accuracyM,
        update.observedAtUnixMs,
    )

    private fun root() = Files.createTempDirectory("cargo-driver-queue").toFile()

    @Test
    fun positions_recorded_offline_survive_restart_in_order() = runBlocking<Unit> {
        val root = root()
        val queue = PendingOutboundQueue(root)
        queue.enqueueLocation(position(100))
        queue.enqueueLocation(position(200))
        queue.enqueueLocation(position(150))

        val reopened = PendingOutboundQueue(root)
        assertEquals(2, reopened.pendingCount())
        assertEquals(100L, reopened.oldestLocation()?.observedAtUnixMs)
        reopened.removeLocationThrough(100)
        assertEquals(200L, reopened.oldestLocation()?.observedAtUnixMs)
        reopened.removeLocationThrough(200)
        assertEquals(null, reopened.oldestLocation())
    }

    @Test
    fun the_track_is_bounded_and_drops_the_oldest_positions() = runBlocking<Unit> {
        val queue = PendingOutboundQueue(root())
        for (at in 1L..(PendingOutboundQueue.MAX_TRACK + 3L)) {
            queue.enqueueLocation(position(at))
        }
        assertEquals(PendingOutboundQueue.MAX_TRACK, queue.pendingCount())
        assertEquals(4L, queue.oldestLocation()?.observedAtUnixMs)
    }

    @Test
    fun a_queue_written_by_the_previous_version_keeps_its_position() = runBlocking<Unit> {
        val root = root()
        File(root, "queue.json").writeText(
            """{"location":{"latitude":49.84,"longitude":24.03,"observed_at_unix_ms":77},"media":[]}""",
        )
        assertEquals(77L, PendingOutboundQueue(root).oldestLocation()?.observedAtUnixMs)
    }

    @Test
    fun sending_keeps_the_rest_of_the_track_when_the_connection_drops() = runBlocking<Unit> {
        val queue = PendingOutboundQueue(root())
        for (at in 1L..3L) queue.enqueueLocation(position(at))
        val sent = mutableListOf<Long>()

        val last = OutboundSender.sendLocations(queue) { update ->
            if (update.observedAtUnixMs == 2L) throw IOException("offline")
            sent += update.observedAtUnixMs
            accepted(update)
        }

        assertEquals(listOf(1L), sent)
        assertEquals(1L, last?.observedAtUnixMs)
        assertEquals(2, queue.pendingCount())
    }

    @Test
    fun a_refused_position_is_skipped_and_an_expired_link_keeps_the_track() = runBlocking<Unit> {
        val queue = PendingOutboundQueue(root())
        for (at in 1L..3L) queue.enqueueLocation(position(at))

        OutboundSender.sendLocations(queue) { update ->
            when (update.observedAtUnixMs) {
                1L -> throw ControlApiException(409, "older")
                2L -> throw ControlApiException(404, "gone")
                else -> accepted(update)
            }
        }

        assertEquals(2L, queue.oldestLocation()?.observedAtUnixMs)
        assertEquals(2, queue.pendingCount())
    }

    @Test
    fun media_payload_and_stable_document_id_survive_restart() = runBlocking<Unit> {
        val root = root()
        val queue = PendingOutboundQueue(root)
        val content = byteArrayOf(1, 2, 3, 4)
        val upload = PickedUpload("proof.jpg", "image/jpeg") { content.inputStream() }
        val documentId = "0123456789abcdef0123456789abcdef"

        queue.enqueueMedia(
            documentId = documentId,
            upload = upload,
            pointId = "51515151515151515151515151515151",
            createdAtUnixMs = 300,
        )

        val reopened = PendingOutboundQueue(root)
        val pending = reopened.pendingMedia().single()
        assertEquals(documentId, pending.documentId)
        assertEquals(upload.name, pending.originalName)
        assertEquals(upload.mimeType, pending.mimeType)
        assertContentEquals(content, reopened.mediaFile(pending)?.readBytes())
        assertEquals(1, reopened.pendingCount())

        reopened.removeMedia(documentId)
        assertEquals(0, reopened.pendingCount())
        assertEquals(emptyList(), reopened.pendingMedia())
    }

    @Test
    fun a_refused_file_stays_visible_until_the_driver_decides() = runBlocking<Unit> {
        val root = root()
        val queue = PendingOutboundQueue(root)
        val documentId = "0123456789abcdef0123456789abcdef"
        queue.enqueueMedia(
            documentId,
            PickedUpload("big.mp4", "video/mp4") { byteArrayOf(1).inputStream() },
            null,
            1,
        )
        queue.markRejected(documentId, 413)

        val reopened = PendingOutboundQueue(root)
        assertEquals(0, reopened.pendingCount())
        assertEquals(emptyList(), reopened.pendingMedia())
        assertEquals(413, reopened.rejectedMedia().single().rejectedStatus)

        reopened.retryRejected(documentId)
        assertEquals(1, reopened.pendingCount())
        assertEquals(emptyList(), reopened.rejectedMedia())
    }
}
