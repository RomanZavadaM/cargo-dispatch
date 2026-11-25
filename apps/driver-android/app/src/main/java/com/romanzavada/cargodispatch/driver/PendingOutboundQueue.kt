package com.romanzavada.cargodispatch.driver

import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import java.io.File
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
)

data class QueuedMediaUpload(
    val metadata: PendingMediaMetadata,
    val bytes: ByteArray,
)

@Serializable
private data class PendingOutboundState(
    val location: LocationUpdate? = null,
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

    suspend fun enqueueLocation(update: LocationUpdate) = withContext(Dispatchers.IO) {
        synchronized(lock) {
            val current = state.location
            if (current == null || update.observedAtUnixMs >= current.observedAtUnixMs) {
                state = state.copy(location = update)
                saveState()
            }
        }
    }

    suspend fun pendingLocation(): LocationUpdate? = withContext(Dispatchers.IO) {
        synchronized(lock) { state.location }
    }

    suspend fun removeLocationThrough(observedAtUnixMs: Long) = withContext(Dispatchers.IO) {
        synchronized(lock) {
            val current = state.location ?: return@synchronized
            if (current.observedAtUnixMs <= observedAtUnixMs) {
                state = state.copy(location = null)
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
            temporary.writeBytes(upload.bytes)
            if (!temporary.renameTo(payload)) {
                temporary.delete()
                error("Cannot persist pending media")
            }

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

    suspend fun pendingMedia(): List<QueuedMediaUpload> = withContext(Dispatchers.IO) {
        synchronized(lock) {
            state.media.mapNotNull { entry ->
                val payload = File(mediaDirectory, entry.payloadFile)
                if (payload.isFile) {
                    QueuedMediaUpload(entry, payload.readBytes())
                } else {
                    null
                }
            }
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

    suspend fun pendingCount(): Int = withContext(Dispatchers.IO) {
        synchronized(lock) {
            state.media.size + if (state.location == null) 0 else 1
        }
    }

    private fun loadState(): PendingOutboundState {
        if (!metadata.isFile) return PendingOutboundState()
        return runCatching {
            json.decodeFromString<PendingOutboundState>(metadata.readText())
        }.getOrDefault(PendingOutboundState())
    }

    private fun saveState() {
        root.mkdirs()
        val temporary = File(root, "queue.json.tmp")
        temporary.writeText(json.encodeToString(state))
        if (!temporary.renameTo(metadata)) {
            temporary.delete()
            error("Cannot persist outbound queue")
        }
    }
}
