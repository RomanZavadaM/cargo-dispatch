package com.romanzavada.cargodispatch.driver

import com.romanzavada.cargodispatch.driver.model.UploadStatus
import com.romanzavada.cargodispatch.driver.net.ControlApiException
import java.io.File
import java.io.IOException
import java.io.RandomAccessFile
import java.security.MessageDigest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** What a file transfer needs from Control. */
interface FileTransferApi {
    suspend fun uploadStatus(documentId: String): UploadStatus

    suspend fun uploadPart(
        entry: PendingMediaMetadata,
        offset: Long,
        total: Long,
        part: ByteArray,
        sha256: String,
    ): UploadStatus

    /** The one request upload of Control 0.3.0 and earlier. */
    suspend fun uploadWhole(entry: PendingMediaMetadata, file: File)

    /**
     * Appends the part of a document that starts at [offset] to [destination].
     * Returns false when Control sent the whole file instead, which replaces
     * what [destination] held.
     */
    suspend fun downloadPart(
        documentId: String,
        offset: Long,
        length: Int,
        destination: File,
    ): Boolean
}

/**
 * Files travel in parts, so a broken connection costs one part, not the file.
 * The sender asks Control what it already holds and goes on from there.
 */
object FileTransfer {
    const val PART_BYTES = 1024 * 1024

    suspend fun upload(
        api: FileTransferApi,
        entry: PendingMediaMetadata,
        file: File,
        partBytes: Int = PART_BYTES,
    ) = withContext(Dispatchers.IO) {
        val total = file.length()
        var status = try {
            api.uploadStatus(entry.documentId)
        } catch (problem: ControlApiException) {
            if (problem.statusCode != 404 && problem.statusCode != 405) throw problem
            api.uploadWhole(entry, file)
            return@withContext
        }
        var misfits = 0
        while (!status.complete) {
            val offset = status.received
            if (offset < 0 || offset > total) throw ControlApiException(409, "another file")
            val part = ByteArray(minOf(partBytes.toLong(), total - offset).toInt())
            RandomAccessFile(file, "r").use { source ->
                source.seek(offset)
                source.readFully(part)
            }
            status = try {
                api.uploadPart(entry, offset, total, part, sha256Hex(part))
            } catch (problem: ControlApiException) {
                // Control holds something else than expected: ask once more.
                if (problem.statusCode != 409 || ++misfits > 1) throw problem
                api.uploadStatus(entry.documentId)
            }
            if (!status.complete && status.received <= offset && misfits == 0) {
                throw IOException("Control did not accept the part")
            }
        }
    }

    /**
     * Completes [partial] with the rest of a document of [size] bytes. A file
     * that does not match [sha256] is thrown away.
     */
    suspend fun download(
        api: FileTransferApi,
        documentId: String,
        size: Long,
        sha256: String?,
        partial: File,
        partBytes: Int = PART_BYTES,
    ) = withContext(Dispatchers.IO) {
        partial.parentFile?.mkdirs()
        if (partial.length() > size) partial.delete()
        if (!partial.isFile) partial.createNewFile()
        while (partial.length() < size) {
            val held = partial.length()
            val length = minOf(partBytes.toLong(), size - held).toInt()
            if (!api.downloadPart(documentId, held, length, partial)) break
            if (partial.length() <= held) throw IOException("Control sent no data")
        }
        val complete = partial.length() == size &&
            (sha256.isNullOrBlank() || sha256Hex(partial).equals(sha256, ignoreCase = true))
        if (!complete) {
            partial.delete()
            throw IOException("The received file does not match")
        }
    }

    fun sha256Hex(bytes: ByteArray): String =
        hex(MessageDigest.getInstance("SHA-256").digest(bytes))

    fun sha256Hex(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        val buffer = ByteArray(64 * 1024)
        file.inputStream().use { source ->
            while (true) {
                val read = source.read(buffer)
                if (read < 0) break
                digest.update(buffer, 0, read)
            }
        }
        return hex(digest.digest())
    }

    private fun hex(bytes: ByteArray): String =
        bytes.joinToString(separator = "") { "%02x".format(it.toInt() and 0xff) }
}
