package com.romanzavada.cargodispatch.driver

import com.romanzavada.cargodispatch.driver.model.UploadStatus
import com.romanzavada.cargodispatch.driver.net.ControlApiException
import java.io.File
import java.io.IOException
import java.nio.file.Files
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlinx.coroutines.runBlocking
import org.junit.Test

class FileTransferTest {
    private val content = ByteArray(2_500) { (it % 251).toByte() }
    private val entry = PendingMediaMetadata(
        documentId = "0123456789abcdef0123456789abcdef",
        originalName = "video.mp4",
        mimeType = "video/mp4",
        createdAtUnixMs = 1,
        payloadFile = "video.bin",
    )

    private class FakeControl(
        private val document: ByteArray = ByteArray(0),
        private val old: Boolean = false,
    ) : FileTransferApi {
        var held = ByteArray(0)
        var total = -1L
        var wholeUploads = 0
        val offsets = mutableListOf<Long>()
        var failAt: Long? = null

        override suspend fun uploadStatus(documentId: String): UploadStatus {
            if (old) throw ControlApiException(404, "no such route")
            return UploadStatus(held.size.toLong(), held.size.toLong() == total)
        }

        override suspend fun uploadPart(
            entry: PendingMediaMetadata,
            offset: Long,
            total: Long,
            part: ByteArray,
            sha256: String,
        ): UploadStatus {
            if (offset == failAt) {
                failAt = null
                throw IOException("connection lost")
            }
            assertEquals(FileTransfer.sha256Hex(part), sha256)
            if (offset != held.size.toLong()) throw ControlApiException(409, "gap")
            offsets += offset
            this.total = total
            held += part
            return UploadStatus(held.size.toLong(), held.size.toLong() == total)
        }

        override suspend fun uploadWhole(entry: PendingMediaMetadata, file: File) {
            wholeUploads += 1
            held = file.readBytes()
        }

        override suspend fun downloadPart(
            documentId: String,
            offset: Long,
            length: Int,
            destination: File,
        ): Boolean {
            if (offset == failAt) {
                failAt = null
                throw IOException("connection lost")
            }
            if (old) {
                destination.writeBytes(document)
                return false
            }
            offsets += offset
            destination.appendBytes(document.copyOfRange(offset.toInt(), offset.toInt() + length))
            return true
        }
    }

    private fun directory(): File = Files.createTempDirectory("transfer").toFile()

    private fun source(): File = File(directory(), "source.bin").apply { writeBytes(content) }

    @Test
    fun a_broken_upload_continues_where_it_stopped() = runBlocking<Unit> {
        val control = FakeControl().apply { failAt = 1_000 }
        val file = source()

        assertFailsWith<IOException> { FileTransfer.upload(control, entry, file, 1_000) }
        FileTransfer.upload(control, entry, file, 1_000)

        assertEquals(listOf(0L, 1_000L, 2_000L), control.offsets)
        assertContentEquals(content, control.held)
    }

    @Test
    fun an_older_control_receives_the_file_whole() = runBlocking<Unit> {
        val control = FakeControl(old = true)

        FileTransfer.upload(control, entry, source(), 1_000)

        assertEquals(1, control.wholeUploads)
        assertContentEquals(content, control.held)
    }

    @Test
    fun a_broken_download_continues_and_is_checked() = runBlocking<Unit> {
        val control = FakeControl(content).apply { failAt = 1_000 }
        val partial = File(directory(), "document.part")
        val checksum = FileTransfer.sha256Hex(content)
        val size = content.size.toLong()

        assertFailsWith<IOException> {
            FileTransfer.download(control, "id", size, checksum, partial, 1_000)
        }
        FileTransfer.download(control, "id", size, checksum, partial, 1_000)

        assertEquals(listOf(0L, 1_000L, 2_000L), control.offsets)
        assertContentEquals(content, partial.readBytes())
    }

    @Test
    fun a_download_with_another_checksum_is_thrown_away() = runBlocking<Unit> {
        val partial = File(directory(), "document.part")

        assertFailsWith<IOException> {
            FileTransfer.download(
                FakeControl(content), "id", content.size.toLong(), "00".repeat(32), partial, 1_000,
            )
        }

        assertFalse(partial.exists())
    }

    @Test
    fun an_older_control_sends_the_whole_document() = runBlocking<Unit> {
        val partial = File(directory(), "document.part")

        FileTransfer.download(
            FakeControl(content, old = true), "id", content.size.toLong(), null, partial, 1_000,
        )

        assertContentEquals(content, partial.readBytes())
    }
}
