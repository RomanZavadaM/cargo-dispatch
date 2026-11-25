package com.romanzavada.cargodispatch.driver

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.romanzavada.cargodispatch.driver.core.RouteOrderPlanner
import com.romanzavada.cargodispatch.driver.core.SessionLink
import com.romanzavada.cargodispatch.driver.model.DriverTrip
import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import com.romanzavada.cargodispatch.driver.net.ControlApi
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
    val error: String? = null,
)

class DriverViewModel : ViewModel() {
    private val mutableState = MutableStateFlow(DriverUiState())
    val state: StateFlow<DriverUiState> = mutableState.asStateFlow()

    private var api: ControlApi? = null

    fun setDriverUrl(value: String) {
        mutableState.value = mutableState.value.copy(driverUrl = value, error = null)
    }

    fun setEmulatorMode(value: Boolean) {
        mutableState.value = mutableState.value.copy(emulatorMode = value, error = null)
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
            api = client
            launchRequest {
                mutableState.value.copy(
                    connectedBaseUrl = target.baseUrl,
                    trip = client.trip(),
                    error = null,
                )
            }
        }
    }

    fun refresh() {
        val client = api ?: return
        launchRequest {
            mutableState.value.copy(trip = client.trip(), error = null)
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
        launchRequest {
            val accepted = client.sendLocation(update)
            val current = mutableState.value
            current.copy(
                trip = current.trip?.copy(latestLocation = accepted),
                error = null,
            )
        }
    }

    fun upload(upload: PickedUpload, pointId: String? = null) {
        val client = api ?: return
        launchRequest {
            mutableState.value.copy(
                trip = client.uploadDocument(
                    originalName = upload.name,
                    mimeType = upload.mimeType,
                    bytes = upload.bytes,
                    pointId = pointId,
                ),
                error = null,
            )
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
