package com.romanzavada.cargodispatch.driver

import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import java.io.File
import java.nio.file.AtomicMoveNotSupportedException
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

@Serializable
data class PendingMediaMetadata(
    val documentId: String,
    val originalName: String,
    val mimeType: String,
    val pointId: String? = null,
    val createdAtUnixMs: Long,
    val payloadFile: String,
    /** HTTP status of a refusal that is not retried until the driver asks. */
    val rejectedStatus: Int? = null,
)

@Serializable
private data class PendingOutboundState(
    // Written by 0.2.0 and earlier: only the newest position. Read once and moved to [track].
    val location: LocationUpdate? = null,
    val track: List<LocationUpdate> = emptyList(),
    val media: List<PendingMediaMetadata> = emptyList(),
)

class PendingOutboundQueue(
    private val root: File,
) {
    private val json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
        prettyPrint = true
    }
    private val metadata = File(root, "queue.json")
    private val mediaDirectory = File(root, "media")
    private val lock = Any()

    private var state: PendingOutboundState = loadState()

    /** Keeps the positions recorded without a connection, oldest first. */
    suspend fun enqueueLocation(update: LocationUpdate) = withContext(Dispatchers.IO) {
        synchronized(lock) {
            val newest = state.track.lastOrNull()
            if (newest == null || update.observedAtUnixMs > newest.observedAtUnixMs) {
                state = state.copy(track = (state.track + update).takeLast(MAX_TRACK))
                saveState()
            }
        }
    }

    suspend fun oldestLocation(): LocationUpdate? = withContext(Dispatchers.IO) {
        synchronized(lock) { state.track.firstOrNull() }
    }

    suspend fun removeLocationThrough(observedAtUnixMs: Long) = withContext(Dispatchers.IO) {
        synchronized(lock) {
            val remaining = state.track.filter { it.observedAtUnixMs > observedAtUnixMs }
            if (remaining.size != state.track.size) {
                state = state.copy(track = remaining)
                saveState()
            }
        }
    }

    suspend fun enqueueMedia(
        documentId: String,
        upload: PickedUpload,
        pointId: String?,
        createdAtUnixMs: Long,
    ): PendingMediaMetadata = withContext(Dispatchers.IO) {
        synchronized(lock) {
            require(documentId.matches(Regex("[0-9a-f]{32}"))) { "invalid document id" }
            mediaDirectory.mkdirs()
            root.mkdirs()

            state.media.firstOrNull { it.documentId == documentId }?.let { existing ->
                return@synchronized existing
            }

            val payloadName = "$documentId.bin"
            val payload = File(mediaDirectory, payloadName)
            val temporary = File(mediaDirectory, "$payloadName.tmp")
            upload.open().use { source ->
                temporary.outputStream().use { target -> source.copyTo(target) }
            }
            replaceAtomically(temporary, payload)

            val entry = PendingMediaMetadata(
                documentId = documentId,
                originalName = upload.name,
                mimeType = upload.mimeType,
                pointId = pointId,
                createdAtUnixMs = createdAtUnixMs,
                payloadFile = payloadName,
            )
            state = state.copy(media = state.media + entry)
            saveState()
            entry
        }
    }

    /** Files still to be sent. Their content stays on disk: see [mediaFile]. */
    suspend fun pendingMedia(): List<PendingMediaMetadata> = withContext(Dispatchers.IO) {
        synchronized(lock) { state.media.filter { it.rejectedStatus == null } }
    }

    suspend fun rejectedMedia(): List<PendingMediaMetadata> = withContext(Dispatchers.IO) {
        synchronized(lock) { state.media.filter { it.rejectedStatus != null } }
    }

    suspend fun mediaFile(entry: PendingMediaMetadata): File? =
        withContext(Dispatchers.IO) {
            synchronized(lock) {
                File(mediaDirectory, entry.payloadFile).takeIf { it.isFile }
            }
        }

    suspend fun markRejected(documentId: String, status: Int) =
        setRejection(documentId, status)

    suspend fun retryRejected(documentId: String) =
        setRejection(documentId, null)

    private suspend fun setRejection(documentId: String, status: Int?) =
        withContext(Dispatchers.IO) {
            synchronized(lock) {
                state = state.copy(
                    media = state.media.map {
                        if (it.documentId == documentId) it.copy(rejectedStatus = status) else it
                    },
                )
                saveState()
            }
        }

    suspend fun removeMedia(documentId: String) = withContext(Dispatchers.IO) {
        synchronized(lock) {
            val existing = state.media.firstOrNull { it.documentId == documentId }
                ?: return@synchronized
            state = state.copy(media = state.media.filterNot { it.documentId == documentId })
            saveState()
            File(mediaDirectory, existing.payloadFile).delete()
        }
    }

    /** Items waiting to be sent; refused files are listed separately. */
    suspend fun pendingCount(): Int = withContext(Dispatchers.IO) {
        synchronized(lock) {
            state.media.count { it.rejectedStatus == null } + state.track.size
        }
    }

    private fun loadState(): PendingOutboundState {
        if (!metadata.isFile) return PendingOutboundState()
        val loaded = runCatching {
            json.decodeFromString<PendingOutboundState>(metadata.readText())
        }.getOrDefault(PendingOutboundState())
        val legacy = loaded.location ?: return loaded
        return loaded.copy(
            location = null,
            track = (loaded.track + legacy).sortedBy { it.observedAtUnixMs },
        )
    }

    private fun saveState() {
        root.mkdirs()
        val temporary = File(root, "queue.json.tmp")
        temporary.writeText(json.encodeToString(state))
        replaceAtomically(temporary, metadata)
    }

    private fun replaceAtomically(source: File, target: File) {
        try {
            Files.move(
                source.toPath(),
                target.toPath(),
                StandardCopyOption.ATOMIC_MOVE,
                StandardCopyOption.REPLACE_EXISTING,
            )
        } catch (_: AtomicMoveNotSupportedException) {
            Files.move(
                source.toPath(),
                target.toPath(),
                StandardCopyOption.REPLACE_EXISTING,
            )
        }
    }

    companion object {
        /** About two hours of tracking at one position every 15 seconds. */
        const val MAX_TRACK = 500

        private val sessionInstances = ConcurrentHashMap<String, PendingOutboundQueue>()

        fun forSession(baseRoot: File, baseUrl: String, token: String): PendingOutboundQueue {
            val digest = MessageDigest.getInstance("SHA-256")
                .digest((baseUrl.trimEnd('/') + "|" + token).toByteArray(Charsets.UTF_8))
                .joinToString(separator = "") { "%02x".format(it.toInt() and 0xff) }
            val root = File(baseRoot, digest)
            return sessionInstances.computeIfAbsent(root.absolutePath) {
                PendingOutboundQueue(root)
            }
        }
    }
}
