package com.romanzavada.cargodispatch.driver

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.romanzavada.cargodispatch.driver.core.DriverProblem
import com.romanzavada.cargodispatch.driver.model.DriverDocument
import com.romanzavada.cargodispatch.driver.core.RouteOrderPlanner
import com.romanzavada.cargodispatch.driver.core.SessionLink
import com.romanzavada.cargodispatch.driver.core.SessionLinkException
import com.romanzavada.cargodispatch.driver.model.DriverTrip
import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import com.romanzavada.cargodispatch.driver.net.ControlApi
import com.romanzavada.cargodispatch.driver.net.ControlApiException
import java.io.File
import java.io.IOException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class DriverUiState(
    val driverUrl: String = "",
    val emulatorMode: Boolean = false,
    val connectedBaseUrl: String? = null,
    val trip: DriverTrip? = null,
    val busy: Boolean = false,
    val offline: Boolean = false,
    val pendingOutbound: Int = 0,
    val rejected: List<PendingMediaMetadata> = emptyList(),
    val problem: DriverProblem? = null,
)

class DriverViewModel(application: Application) : AndroidViewModel(application) {
    private val mutableState = MutableStateFlow(DriverUiState())
    val state: StateFlow<DriverUiState> = mutableState.asStateFlow()

    private val queueRoot = File(application.filesDir, "outbound-queue")
    private val downloadRoot = File(application.cacheDir, "downloads")
    private var api: ControlApi? = null
    private var pendingQueue: PendingOutboundQueue? = null

    /** Grows with every action of the driver, so a slower background refresh is dropped. */
    private var generation = 0

    fun setDriverUrl(value: String) {
        mutableState.value = mutableState.value.copy(
            driverUrl = value,
            pendingOutbound = 0,
            problem = null,
        )
    }

    fun setEmulatorMode(value: Boolean) {
        mutableState.value = mutableState.value.copy(
            emulatorMode = value,
            pendingOutbound = 0,
            problem = null,
        )
    }

    fun restoreSession(saved: SavedDriverSession) {
        mutableState.value = mutableState.value.copy(
            driverUrl = saved.driverUrl,
            emulatorMode = saved.emulatorMode,
            problem = null,
        )
        connect()
    }

    fun reportProblem(problem: DriverProblem) {
        mutableState.value = mutableState.value.copy(problem = problem)
    }

    fun disconnect() {
        generation += 1
        api = null
        pendingQueue = null
        mutableState.value = DriverUiState(
            driverUrl = mutableState.value.driverUrl,
            emulatorMode = mutableState.value.emulatorMode,
        )
    }

    fun connect() {
        val current = mutableState.value
        val target = try {
            SessionLink.parseDriverUrl(current.driverUrl, current.emulatorMode)
        } catch (invalid: SessionLinkException) {
            mutableState.value = current.copy(problem = invalid.problem)
            return
        }
        val client = ControlApi(target.baseUrl, target.token)
        val queue = PendingOutboundQueue.forSession(queueRoot, target.baseUrl, target.token)
        api = client
        pendingQueue = queue
        request {
            client.trip()
            flush(client, queue)
            val trip = client.trip()
            mutableState.value = mutableState.value.copy(
                connectedBaseUrl = target.baseUrl,
                trip = trip,
            )
        }
    }

    fun refresh() {
        val client = api ?: return
        val queue = pendingQueue ?: return
        request {
            flush(client, queue)
            val trip = client.trip()
            mutableState.value = mutableState.value.copy(trip = trip)
        }
    }

    /** Periodic update while the screen is open. It never blocks the buttons. */
    suspend fun refreshQuietly() {
        val client = api ?: return
        val queue = pendingQueue ?: return
        if (mutableState.value.busy || mutableState.value.trip == null) return
        val startedAt = generation
        try {
            flush(client, queue)
            val trip = client.trip()
            if (generation != startedAt || api !== client) return
            val current = mutableState.value
            mutableState.value = current.copy(
                trip = trip,
                offline = false,
                // A message about a lost connection is no longer true.
                problem = current.problem.takeUnless { it == DriverProblem.NoConnection },
            )
        } catch (_: IOException) {
            if (generation == startedAt) {
                mutableState.value = mutableState.value.copy(offline = true)
            }
        } catch (failure: ControlApiException) {
            if (generation == startedAt) {
                mutableState.value = mutableState.value.copy(
                    problem = DriverProblem.fromStatus(failure.statusCode),
                )
            }
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: RuntimeException) {
            // An unreadable answer must not close the application; the next round retries.
        }
        updateQueueState(queue)
    }

    fun complete(pointId: String) {
        val client = api ?: return
        request {
            val trip = client.complete(pointId)
            mutableState.value = mutableState.value.copy(trip = trip)
        }
    }

    fun moveOpen(pointId: String, delta: Int) {
        val client = api ?: return
        val trip = mutableState.value.trip ?: return
        val order = RouteOrderPlanner.movedOpenPointIds(trip.points, pointId, delta) ?: return
        request {
            val updated = client.reorder(trip.localRouteRevision, order)
            mutableState.value = mutableState.value.copy(trip = updated)
        }
    }

    fun sendLocation(update: LocationUpdate) {
        val client = api ?: return
        val queue = pendingQueue ?: return
        request {
            queue.enqueueLocation(update)
            flush(client, queue)
            val trip = client.trip()
            mutableState.value = mutableState.value.copy(trip = trip)
        }
    }

    fun upload(upload: PickedUpload, pointId: String? = null) {
        val client = api ?: return
        val queue = pendingQueue ?: return
        request {
            val known = mutableState.value.trip?.points?.any { it.pointId == pointId } == true
            try {
                queue.enqueueMedia(
                    documentId = ControlApi.newDocumentId(),
                    upload = upload,
                    pointId = pointId.takeIf { known },
                    createdAtUnixMs = System.currentTimeMillis(),
                )
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                reportProblem(DriverProblem.FileUnreadable)
                return@request
            }
            flush(client, queue)
            val trip = client.trip()
            mutableState.value = mutableState.value.copy(trip = trip)
        }
    }

    fun retryRejected(documentId: String) {
        val queue = pendingQueue ?: return
        viewModelScope.launch {
            queue.retryRejected(documentId)
            updateQueueState(queue)
            refresh()
        }
    }

    fun discardRejected(documentId: String) {
        val queue = pendingQueue ?: return
        viewModelScope.launch {
            queue.removeMedia(documentId)
            updateQueueState(queue)
        }
    }

    /** Receives a document in parts; saving it again continues a broken transfer. */
    fun download(document: DriverDocument, onResult: suspend (File) -> Unit) {
        val client = api ?: return
        request {
            val partial = File(downloadRoot, "${document.documentId}.part")
            FileTransfer.download(client, document.documentId, document.size, document.sha256, partial)
            onResult(partial)
            partial.delete()
        }
    }

    private suspend fun flush(client: ControlApi, queue: PendingOutboundQueue) {
        OutboundSender.sendLocations(queue, client::sendLocation)
        OutboundSender.sendMedia(client, queue)
    }

    private suspend fun updateQueueState(queue: PendingOutboundQueue) {
        val pending = queue.pendingCount()
        val rejected = queue.rejectedMedia()
        if (pendingQueue === queue) {
            mutableState.value = mutableState.value.copy(
                pendingOutbound = pending,
                rejected = rejected,
            )
        }
    }

    /** One action of the driver: shows progress and turns failures into a message. */
    private fun request(block: suspend () -> Unit) {
        generation += 1
        val queue = pendingQueue
        mutableState.value = mutableState.value.copy(busy = true, problem = null)
        viewModelScope.launch {
            var problem: DriverProblem? = null
            var offline = false
            try {
                block()
            } catch (_: IOException) {
                problem = DriverProblem.NoConnection
                offline = true
            } catch (failure: ControlApiException) {
                problem = DriverProblem.fromStatus(failure.statusCode)
                if (problem == DriverProblem.RouteChanged) {
                    runCatching { api?.trip() }.getOrNull()?.let { trip ->
                        mutableState.value = mutableState.value.copy(trip = trip)
                    }
                }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: RuntimeException) {
                problem = DriverProblem.Unknown
            }
            queue?.let { updateQueueState(it) }
            mutableState.value = mutableState.value.copy(
                busy = false,
                offline = offline,
                // Keep a message that the action itself reported (for example a save failure).
                problem = problem ?: mutableState.value.problem,
            )
        }
    }
}
