package com.romanzavada.cargodispatch.driver

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.romanzavada.cargodispatch.driver.core.RouteOrderPlanner
import com.romanzavada.cargodispatch.driver.core.SessionLink
import com.romanzavada.cargodispatch.driver.model.DriverTrip
import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import com.romanzavada.cargodispatch.driver.net.ControlApi
import com.romanzavada.cargodispatch.driver.net.ControlApiException
import java.io.File
import java.io.IOException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class DriverUiState(
    val driverUrl: String = "",
    val emulatorMode: Boolean = true,
    val connectedBaseUrl: String? = null,
    val trip: DriverTrip? = null,
    val busy: Boolean = false,
    val pendingOutbound: Int = 0,
    val error: String? = null,
)

class DriverViewModel(application: Application) : AndroidViewModel(application) {
    private val mutableState = MutableStateFlow(DriverUiState())
    val state: StateFlow<DriverUiState> = mutableState.asStateFlow()

    private val queueRoot = File(application.filesDir, "outbound-queue")
    private var api: ControlApi? = null
    private var pendingQueue: PendingOutboundQueue? = null

    fun setDriverUrl(value: String) {
        mutableState.value = mutableState.value.copy(
            driverUrl = value,
            pendingOutbound = 0,
            error = null,
        )
    }

    fun setEmulatorMode(value: Boolean) {
        mutableState.value = mutableState.value.copy(
            emulatorMode = value,
            pendingOutbound = 0,
            error = null,
        )
    }

    fun restoreSession(saved: SavedDriverSession) {
        mutableState.value = mutableState.value.copy(
            driverUrl = saved.driverUrl,
            emulatorMode = saved.emulatorMode,
            error = null,
        )
        connect()
    }

    fun disconnect() {
        api = null
        pendingQueue = null
        mutableState.value = DriverUiState(
            driverUrl = mutableState.value.driverUrl,
            emulatorMode = mutableState.value.emulatorMode,
        )
    }

    fun connect() {
        val current = mutableState.value
        runCatching {
            SessionLink.parseDriverUrl(current.driverUrl, current.emulatorMode)
        }.onFailure {
            mutableState.value = current.copy(error = it.message ?: "Некоректний Driver URL")
        }.onSuccess { target ->
            val client = ControlApi(target.baseUrl, target.token)
            val queue = PendingOutboundQueue.forSession(queueRoot, target.baseUrl, target.token)
            api = client
            pendingQueue = queue
            mutableState.value = current.copy(busy = true, error = null)
            viewModelScope.launch {
                mutableState.value = runCatching {
                    client.trip()
                    flushPending(client, queue)
                    val trip = client.trip()
                    current.copy(
                        connectedBaseUrl = target.baseUrl,
                        trip = trip,
                        pendingOutbound = queue.pendingCount(),
                        error = null,
                    )
                }.getOrElse { problem ->
                    current.copy(
                        pendingOutbound = queue.pendingCount(),
                        error = problem.message ?: problem::class.java.simpleName,
                    )
                }.copy(busy = false)
            }
        }
    }

    fun refresh() {
        val client = api ?: return
        val queue = pendingQueue
        mutableState.value = mutableState.value.copy(busy = true, error = null)
        viewModelScope.launch {
            mutableState.value = runCatching {
                if (queue != null) {
                    flushPending(client, queue)
                }
                mutableState.value.copy(
                    trip = client.trip(),
                    pendingOutbound = queue?.pendingCount() ?: 0,
                    error = null,
                )
            }.getOrElse { problem ->
                mutableState.value.copy(
                    pendingOutbound = queue?.pendingCount() ?: 0,
                    error = problem.message ?: problem::class.java.simpleName,
                )
            }.copy(busy = false)
        }
    }

    fun complete(pointId: String) {
        val client = api ?: return
        launchRequest {
            mutableState.value.copy(trip = client.complete(pointId), error = null)
        }
    }

    fun moveOpen(pointId: String, delta: Int) {
        val client = api ?: return
        val trip = mutableState.value.trip ?: return
        val order = RouteOrderPlanner.movedOpenPointIds(trip.points, pointId, delta) ?: return
        launchRequest {
            mutableState.value.copy(
                trip = client.reorder(trip.localRouteRevision, order),
                error = null,
            )
        }
    }

    fun sendLocation(update: LocationUpdate) {
        val client = api ?: return
        val queue = pendingQueue ?: return
        mutableState.value = mutableState.value.copy(busy = true, error = null)
        viewModelScope.launch {
            runCatching {
                queue.enqueueLocation(update)
                val accepted = client.sendLocation(update)
                queue.removeLocationThrough(accepted.observedAtUnixMs)
                val current = mutableState.value
                current.copy(
                    trip = current.trip?.copy(latestLocation = accepted),
                    pendingOutbound = queue.pendingCount(),
                    error = null,
                )
            }.onSuccess {
                mutableState.value = it.copy(busy = false)
            }.onFailure { problem ->
                when (problem) {
                    is IOException -> {
                        mutableState.value = mutableState.value.copy(
                            busy = false,
                            pendingOutbound = queue.pendingCount(),
                            error = null,
                        )
                    }
                    is ControlApiException -> {
                        if (problem.statusCode == 409) {
                            queue.removeLocationThrough(update.observedAtUnixMs)
                        }
                        val refreshed = runCatching { client.trip() }.getOrNull()
                        mutableState.value = mutableState.value.copy(
                            busy = false,
                            trip = refreshed ?: mutableState.value.trip,
                            pendingOutbound = queue.pendingCount(),
                            error = if (problem.statusCode == 409) null else problem.message,
                        )
                    }
                    else -> {
                        mutableState.value = mutableState.value.copy(
                            busy = false,
                            pendingOutbound = queue.pendingCount(),
                            error = problem.message ?: problem::class.java.simpleName,
                        )
                    }
                }
            }
        }
    }

    fun upload(upload: PickedUpload, pointId: String? = null) {
        val client = api ?: return
        val queue = pendingQueue ?: return
        mutableState.value = mutableState.value.copy(busy = true, error = null)
        viewModelScope.launch {
            val documentId = ControlApi.newDocumentId()
            val createdAtUnixMs = System.currentTimeMillis()
            runCatching {
                queue.enqueueMedia(
                    documentId = documentId,
                    upload = upload,
                    pointId = pointId,
                    createdAtUnixMs = createdAtUnixMs,
                )
                val trip = client.uploadDocument(
                    documentId = documentId,
                    originalName = upload.name,
                    mimeType = upload.mimeType,
                    bytes = upload.bytes,
                    pointId = pointId,
                    createdAtUnixMs = createdAtUnixMs,
                )
                queue.removeMedia(documentId)
                mutableState.value.copy(
                    trip = trip,
                    pendingOutbound = queue.pendingCount(),
                    error = null,
                )
            }.onSuccess {
                mutableState.value = it.copy(busy = false)
            }.onFailure { problem ->
                mutableState.value = mutableState.value.copy(
                    busy = false,
                    pendingOutbound = queue.pendingCount(),
                    error = if (problem is IOException) null
                    else problem.message ?: problem::class.java.simpleName,
                )
            }
        }
    }

    fun download(documentId: String, onResult: (Result<ByteArray>) -> Unit) {
        val client = api ?: return
        mutableState.value = mutableState.value.copy(busy = true, error = null)
        viewModelScope.launch {
            val result = runCatching { client.downloadDocument(documentId) }
            result.exceptionOrNull()?.let { problem ->
                mutableState.value = mutableState.value.copy(
                    error = problem.message ?: problem::class.java.simpleName,
                )
            }
            mutableState.value = mutableState.value.copy(busy = false)
            onResult(result)
        }
    }

    private suspend fun flushPending(
        client: ControlApi,
        queue: PendingOutboundQueue,
    ) {
        queue.pendingLocation()?.let { update ->
            try {
                val accepted = client.sendLocation(update)
                queue.removeLocationThrough(accepted.observedAtUnixMs)
            } catch (problem: ControlApiException) {
                if (problem.statusCode == 409) {
                    queue.removeLocationThrough(update.observedAtUnixMs)
                }
            } catch (_: IOException) {
                return
            }
        }

        for (pending in queue.pendingMedia()) {
            val item = pending.metadata
            try {
                client.uploadDocument(
                    documentId = item.documentId,
                    originalName = item.originalName,
                    mimeType = item.mimeType,
                    bytes = pending.bytes,
                    pointId = item.pointId,
                    createdAtUnixMs = item.createdAtUnixMs,
                )
                queue.removeMedia(item.documentId)
            } catch (_: IOException) {
                return
            } catch (_: ControlApiException) {
                // Permanent conflict/rejection remains queued for visible manual attention.
            }
        }
    }

    private fun launchRequest(block: suspend () -> DriverUiState) {
        mutableState.value = mutableState.value.copy(busy = true, error = null)
        viewModelScope.launch {
            mutableState.value = runCatching { block() }
                .getOrElse { problem ->
                    mutableState.value.copy(
                        error = problem.message ?: problem::class.java.simpleName,
                    )
                }
                .copy(busy = false)
        }
    }
}
