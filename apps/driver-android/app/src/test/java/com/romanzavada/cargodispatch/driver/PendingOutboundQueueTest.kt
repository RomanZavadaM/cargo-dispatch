package com.romanzavada.cargodispatch.driver

import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import java.nio.file.Files
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlinx.coroutines.runBlocking
import org.junit.Test

class PendingOutboundQueueTest {
    @Test
    fun newest_location_replaces_older_pending_sample_and_survives_restart() = runBlocking {
        val root = Files.createTempDirectory("cargo-driver-queue").toFile()
        val queue = PendingOutboundQueue(root)

        val newer = LocationUpdate(49.84, 24.03, 6.0, 200)
        queue.enqueueLocation(newer)
        queue.enqueueLocation(LocationUpdate(49.83, 24.02, 8.0, 100))

        assertEquals(newer, queue.pendingLocation())
        assertEquals(1, queue.pendingCount())

        val reopened = PendingOutboundQueue(root)
        assertEquals(newer, reopened.pendingLocation())

        reopened.removeLocationThrough(199)
        assertEquals(newer, reopened.pendingLocation())
        reopened.removeLocationThrough(200)
        assertEquals(null, reopened.pendingLocation())
    }

    @Test
    fun media_payload_and_stable_document_id_survive_restart() = runBlocking {
        val root = Files.createTempDirectory("cargo-driver-media").toFile()
        val queue = PendingOutboundQueue(root)
        val upload = PickedUpload(
            name = "proof.jpg",
            mimeType = "image/jpeg",
            bytes = byteArrayOf(1, 2, 3, 4),
        )
        val documentId = "0123456789abcdef0123456789abcdef"

        queue.enqueueMedia(
            documentId = documentId,
            upload = upload,
            pointId = "51515151515151515151515151515151",
            createdAtUnixMs = 300,
        )

        val reopened = PendingOutboundQueue(root)
        val pending = reopened.pendingMedia().single()
        assertEquals(documentId, pending.metadata.documentId)
        assertEquals(upload.name, pending.metadata.originalName)
        assertEquals(upload.mimeType, pending.metadata.mimeType)
        assertContentEquals(upload.bytes, pending.bytes)
        assertEquals(1, reopened.pendingCount())

        reopened.removeMedia(documentId)
        assertEquals(0, reopened.pendingCount())
        assertEquals(emptyList(), reopened.pendingMedia())
    }
}
