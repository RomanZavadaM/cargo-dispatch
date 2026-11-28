package com.romanzavada.cargodispatch.driver

import android.Manifest
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.repeatOnLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.romanzavada.cargodispatch.driver.core.DriverProblem
import com.romanzavada.cargodispatch.driver.core.FileSize
import com.romanzavada.cargodispatch.driver.core.NavigationLinks
import com.romanzavada.cargodispatch.driver.core.SessionLink
import com.romanzavada.cargodispatch.driver.core.SizeUnit
import com.romanzavada.cargodispatch.driver.model.DriverDocument
import com.romanzavada.cargodispatch.driver.model.DriverPoint
import com.romanzavada.cargodispatch.driver.model.DriverTrip
import java.text.DateFormat
import java.util.Date
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import org.maplibre.android.MapLibre

private const val REFRESH_INTERVAL_MS = 15_000L

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        MapLibre.getInstance(applicationContext)

        val launchSession = intent
            ?.getStringExtra(EXTRA_DRIVER_URL)
            ?.takeIf(String::isNotBlank)
            ?.let { driverUrl ->
                SavedDriverSession(
                    driverUrl = driverUrl,
                    emulatorMode = intent.getBooleanExtra(EXTRA_EMULATOR_MODE, false),
                )
            }

        setContent {
            MaterialTheme {
                DriverScreen(launchSession = launchSession)
            }
        }
    }

    private companion object {
        const val EXTRA_DRIVER_URL = "cargo_dispatch_driver_url"
        const val EXTRA_EMULATOR_MODE = "cargo_dispatch_emulator_mode"
    }
}

@Composable
private fun DriverScreen(
    launchSession: SavedDriverSession? = null,
    viewModel: DriverViewModel = viewModel(),
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val trackingActive by DriverTrackingState.active.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val sessionPreferences = remember { SessionPreferences(context.applicationContext) }
    val locationProvider = remember { ForegroundLocationProvider(context.applicationContext) }
    var pendingSave by remember { mutableStateOf<DriverDocument?>(null) }
    var pendingCapture by remember { mutableStateOf<PendingCapture?>(null) }
    var attachPointId by rememberSaveable { mutableStateOf<String?>(null) }
    var confirmPoint by remember { mutableStateOf<DriverPoint?>(null) }

    fun send(read: () -> PickedUpload) {
        runCatching(read)
            .onSuccess { viewModel.upload(it, attachPointId) }
            .onFailure { viewModel.reportProblem(DriverProblem.FileUnreadable) }
    }

    val saveDocument = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("*/*"),
    ) { uri ->
        val selected = pendingSave
        pendingSave = null
        if (uri != null && selected != null) {
            viewModel.download(selected) { file ->
                runCatching {
                    withContext(Dispatchers.IO) {
                        context.contentResolver.openOutputStream(uri, "w")?.use { output ->
                            file.inputStream().use { it.copyTo(output) }
                        } ?: error("Cannot open destination")
                    }
                }.onSuccess {
                    Toast.makeText(context, R.string.document_saved, Toast.LENGTH_SHORT).show()
                }.onFailure {
                    viewModel.reportProblem(DriverProblem.FileNotSaved)
                }
            }
        }
    }

    val filePicker = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri != null) {
            send { context.contentResolver.readUpload(uri) }
        }
    }

    val mediaPicker = rememberLauncherForActivityResult(
        ActivityResultContracts.PickVisualMedia(),
    ) { uri ->
        if (uri != null) {
            send { context.contentResolver.readUpload(uri) }
        }
    }

    val takePhoto = rememberLauncherForActivityResult(
        ActivityResultContracts.TakePicture(),
    ) { success ->
        val capture = pendingCapture
        pendingCapture = null
        if (success && capture != null) {
            send { capture.readUpload(context) }
        }
    }

    val captureVideo = rememberLauncherForActivityResult(
        ActivityResultContracts.CaptureVideo(),
    ) { success ->
        val capture = pendingCapture
        pendingCapture = null
        if (success && capture != null) {
            send { capture.readUpload(context) }
        }
    }

    val locationPermission = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        if (granted) {
            locationProvider.current { result ->
                result
                    .onSuccess(viewModel::sendLocation)
                    .onFailure { viewModel.reportProblem(DriverProblem.LocationUnavailable) }
            }
        } else {
            viewModel.reportProblem(DriverProblem.LocationPermission)
        }
    }

    val startTrackingService = {
        ContextCompat.startForegroundService(
            context,
            Intent(context, DriverTrackingService::class.java),
        )
    }

    val trackingLocationPermissions = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { grants ->
        val allowed =
            grants[Manifest.permission.ACCESS_FINE_LOCATION] == true ||
                grants[Manifest.permission.ACCESS_COARSE_LOCATION] == true
        if (allowed) {
            startTrackingService()
        } else {
            viewModel.reportProblem(DriverProblem.LocationPermission)
        }
    }

    val trackingNotificationPermission = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) {
        trackingLocationPermissions.launch(
            arrayOf(
                Manifest.permission.ACCESS_COARSE_LOCATION,
                Manifest.permission.ACCESS_FINE_LOCATION,
            ),
        )
    }

    val requestTrackingStart = {
        val needsNotificationPermission =
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                ContextCompat.checkSelfPermission(
                    context,
                    Manifest.permission.POST_NOTIFICATIONS,
                ) != PackageManager.PERMISSION_GRANTED

        if (needsNotificationPermission) {
            trackingNotificationPermission.launch(Manifest.permission.POST_NOTIFICATIONS)
        } else {
            trackingLocationPermissions.launch(
                arrayOf(
                    Manifest.permission.ACCESS_COARSE_LOCATION,
                    Manifest.permission.ACCESS_FINE_LOCATION,
                ),
            )
        }
    }

    val stopTracking: () -> Unit = {
        context.stopService(Intent(context, DriverTrackingService::class.java))
        Unit
    }

    LaunchedEffect(Unit) {
        (launchSession ?: sessionPreferences.load())?.let(viewModel::restoreSession)
    }

    LaunchedEffect(state.connectedBaseUrl) {
        if (state.connectedBaseUrl != null) {
            sessionPreferences.save(state.driverUrl, state.emulatorMode)
        }
    }

    DisposableEffect(lifecycleOwner, state.connectedBaseUrl) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME && state.connectedBaseUrl != null) {
                viewModel.refresh()
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
        }
    }

    // New points, documents and the dispatcher's changes appear by themselves.
    LaunchedEffect(lifecycleOwner, state.connectedBaseUrl) {
        if (state.connectedBaseUrl != null) {
            lifecycleOwner.lifecycle.repeatOnLifecycle(Lifecycle.State.RESUMED) {
                while (true) {
                    delay(REFRESH_INTERVAL_MS)
                    viewModel.refreshQuietly()
                }
            }
        }
    }

    confirmPoint?.let { point ->
        AlertDialog(
            onDismissRequest = { confirmPoint = null },
            title = { Text(stringResource(R.string.confirm_done_title)) },
            text = { Text(stringResource(R.string.confirm_done_text, point.sequence, point.title)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmPoint = null
                        viewModel.complete(point.pointId)
                    },
                ) {
                    Text(stringResource(R.string.confirm_done_button))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmPoint = null }) {
                    Text(stringResource(R.string.cancel))
                }
            },
        )
    }

    Scaffold(modifier = Modifier.fillMaxSize()) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item {
                Text(stringResource(R.string.screen_title), style = MaterialTheme.typography.headlineSmall)
                Text(
                    stringResource(R.string.screen_subtitle),
                    style = MaterialTheme.typography.bodyMedium,
                )
            }

            item {
                ConnectionCard(
                    state = state,
                    setDriverUrl = viewModel::setDriverUrl,
                    setEmulatorMode = viewModel::setEmulatorMode,
                    connect = viewModel::connect,
                    refresh = viewModel::refresh,
                    disconnect = {
                        sessionPreferences.clear()
                        viewModel.disconnect()
                    },
                )
            }

            state.problem?.let { problem ->
                item {
                    Text(
                        stringResource(problemText(problem)),
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            }

            if (state.rejected.isNotEmpty()) {
                item {
                    RejectedCard(
                        rejected = state.rejected,
                        busy = state.busy,
                        retry = viewModel::retryRejected,
                        discard = viewModel::discardRejected,
                    )
                }
            }

            state.trip?.let { trip ->
                item {
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(Modifier.padding(14.dp)) {
                            Text(
                                stringResource(R.string.trip_title, trip.tripId.take(8)),
                                style = MaterialTheme.typography.titleMedium,
                            )
                            Text(
                                stringResource(
                                    R.string.points_progress,
                                    trip.points.count { it.completed },
                                    trip.points.size,
                                ),
                            )
                        }
                    }
                }

                item {
                    DriverMapCard(trip)
                }

                item {
                    LocationCard(
                        state = state,
                        trackingActive = trackingActive,
                        requestLocation = {
                            locationPermission.launch(Manifest.permission.ACCESS_FINE_LOCATION)
                        },
                        startTracking = requestTrackingStart,
                        stopTracking = stopTracking,
                    )
                }

                item {
                    DocumentsCard(
                        trip = trip,
                        busy = state.busy,
                        attachPointId = attachPointId,
                        attach = { attachPointId = it },
                        pickFile = { filePicker.launch(arrayOf("*/*")) },
                        pickMedia = {
                            mediaPicker.launch(
                                PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageAndVideo),
                            )
                        },
                        capturePhoto = {
                            val capture = createCapture(context, CaptureKind.Photo)
                            pendingCapture = capture
                            takePhoto.launch(capture.uri)
                        },
                        captureVideo = {
                            val capture = createCapture(context, CaptureKind.Video)
                            pendingCapture = capture
                            captureVideo.launch(capture.uri)
                        },
                        save = { selected ->
                            pendingSave = selected
                            saveDocument.launch(selected.originalName)
                        },
                    )
                }

                item {
                    Text(stringResource(R.string.route_title), style = MaterialTheme.typography.titleLarge)
                }

                items(trip.points, key = { it.pointId }) { point ->
                    PointCard(
                        point = point,
                        busy = state.busy,
                        navigate = { openNavigation(context, point) },
                        complete = { confirmPoint = point },
                        moveUp = { viewModel.moveOpen(point.pointId, -1) },
                        moveDown = { viewModel.moveOpen(point.pointId, 1) },
                    )
                }
            }
        }
    }
}

private fun problemText(problem: DriverProblem): Int = when (problem) {
    DriverProblem.LinkInvalid -> R.string.problem_link_invalid
    DriverProblem.LinkScheme -> R.string.problem_link_scheme
    DriverProblem.LinkShape -> R.string.problem_link_shape
    DriverProblem.NoConnection -> R.string.problem_no_connection
    DriverProblem.LinkExpired -> R.string.problem_link_expired
    DriverProblem.RouteChanged -> R.string.problem_route_changed
    DriverProblem.FileTooLarge -> R.string.problem_file_too_large
    DriverProblem.Rejected -> R.string.problem_rejected
    DriverProblem.FileUnreadable -> R.string.problem_file_unreadable
    DriverProblem.FileNotSaved -> R.string.problem_file_not_saved
    DriverProblem.LocationPermission -> R.string.tracking_permission_required
    DriverProblem.LocationUnavailable -> R.string.problem_location_unavailable
    DriverProblem.Unknown -> R.string.problem_unknown
}

/** Opens any installed navigator; without one, the destination opens in the browser. */
private fun openNavigation(context: Context, point: DriverPoint) {
    val destination = point.navigation ?: return
    val links = listOf(
        NavigationLinks.geo(destination.latitude, destination.longitude, point.title),
        NavigationLinks.web(destination.latitude, destination.longitude),
    )
    for (link in links) {
        try {
            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(link)))
            return
        } catch (_: ActivityNotFoundException) {
            // Try the next form.
        }
    }
    Toast.makeText(context, R.string.problem_no_navigator, Toast.LENGTH_LONG).show()
}

@Composable
private fun fileSizeText(bytes: Long): String {
    val size = FileSize.of(bytes)
    val format = when (size.unit) {
        SizeUnit.Bytes -> R.string.size_bytes
        SizeUnit.Kilobytes -> R.string.size_kilobytes
        SizeUnit.Megabytes -> R.string.size_megabytes
    }
    return stringResource(format, size.value)
}

private fun pointLabel(point: DriverPoint): String = "${point.sequence}. ${point.title}"

@Composable
private fun ConnectionCard(
    state: DriverUiState,
    setDriverUrl: (String) -> Unit,
    setEmulatorMode: (Boolean) -> Unit,
    connect: () -> Unit,
    refresh: () -> Unit,
    disconnect: () -> Unit,
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(stringResource(R.string.connection_title), style = MaterialTheme.typography.titleMedium)
            OutlinedTextField(
                value = state.driverUrl,
                onValueChange = setDriverUrl,
                modifier = Modifier.fillMaxWidth(),
                label = { Text(stringResource(R.string.driver_url_label)) },
                placeholder = { Text(stringResource(R.string.driver_url_placeholder)) },
                singleLine = true,
            )
            // A link to the device itself only makes sense inside an Android emulator.
            if (SessionLink.isLoopback(state.driverUrl)) {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Checkbox(
                        checked = state.emulatorMode,
                        onCheckedChange = setEmulatorMode,
                    )
                    Column {
                        Text(stringResource(R.string.emulator_label))
                        Text(
                            stringResource(R.string.emulator_hint),
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = connect, enabled = !state.busy && state.driverUrl.isNotBlank()) {
                    Text(if (state.busy) stringResource(R.string.connecting) else stringResource(R.string.connect))
                }
                if (state.trip != null) {
                    Button(onClick = refresh, enabled = !state.busy) {
                        Text(stringResource(R.string.refresh))
                    }
                    OutlinedButton(onClick = disconnect, enabled = !state.busy) {
                        Text(stringResource(R.string.disconnect))
                    }
                }
            }
            if (state.offline) {
                Text(
                    stringResource(R.string.offline),
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                )
            } else if (state.trip != null) {
                Text(stringResource(R.string.connected), style = MaterialTheme.typography.bodySmall)
            }
            if (state.pendingOutbound > 0) {
                Text(
                    stringResource(R.string.pending_outbound, state.pendingOutbound),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
    }
}

@Composable
private fun RejectedCard(
    rejected: List<PendingMediaMetadata>,
    busy: Boolean,
    retry: (String) -> Unit,
    discard: (String) -> Unit,
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(stringResource(R.string.rejected_title), style = MaterialTheme.typography.titleMedium)
            rejected.forEach { item ->
                Text(item.originalName)
                Text(
                    stringResource(
                        problemText(DriverProblem.fromStatus(item.rejectedStatus ?: 400)),
                    ),
                    style = MaterialTheme.typography.bodySmall,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(onClick = { retry(item.documentId) }, enabled = !busy) {
                        Text(stringResource(R.string.retry))
                    }
                    OutlinedButton(onClick = { discard(item.documentId) }, enabled = !busy) {
                        Text(stringResource(R.string.discard))
                    }
                }
                HorizontalDivider()
            }
        }
    }
}

@Composable
private fun LocationCard(
    state: DriverUiState,
    trackingActive: Boolean,
    requestLocation: () -> Unit,
    startTracking: () -> Unit,
    stopTracking: () -> Unit,
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(stringResource(R.string.location_title), style = MaterialTheme.typography.titleMedium)
            val location = state.trip?.latestLocation
            if (location == null) {
                Text(stringResource(R.string.location_missing))
            } else {
                Text(
                    stringResource(
                        R.string.location_value,
                        DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT)
                            .format(Date(location.observedAtUnixMs)),
                        location.accuracyM ?: 0.0,
                    ),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
            Button(onClick = requestLocation, enabled = !state.busy) {
                Text(stringResource(R.string.send_location))
            }
            Text(
                if (trackingActive) {
                    stringResource(R.string.tracking_status_active)
                } else {
                    stringResource(R.string.tracking_status_stopped)
                },
                style = MaterialTheme.typography.bodySmall,
            )
            if (trackingActive) {
                Button(onClick = stopTracking) {
                    Text(stringResource(R.string.tracking_stop))
                }
            } else {
                Button(
                    onClick = startTracking,
                    enabled = !state.busy && state.connectedBaseUrl != null,
                ) {
                    Text(stringResource(R.string.tracking_start))
                }
            }
        }
    }
}

@Composable
private fun DocumentsCard(
    trip: DriverTrip,
    busy: Boolean,
    attachPointId: String?,
    attach: (String?) -> Unit,
    pickFile: () -> Unit,
    pickMedia: () -> Unit,
    capturePhoto: () -> Unit,
    captureVideo: () -> Unit,
    save: (DriverDocument) -> Unit,
) {
    var choosing by remember { mutableStateOf(false) }
    val attached = trip.points.firstOrNull { it.pointId == attachPointId }

    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(stringResource(R.string.documents_title), style = MaterialTheme.typography.titleMedium)
            if (trip.documents.isEmpty()) {
                Text(stringResource(R.string.documents_empty))
            } else {
                trip.documents.forEach { selected ->
                    val point = trip.points.firstOrNull { it.pointId == selected.pointId }
                    Text(selected.originalName)
                    Text(
                        listOfNotNull(fileSizeText(selected.size), point?.let(::pointLabel))
                            .joinToString(" · "),
                        style = MaterialTheme.typography.bodySmall,
                    )
                    Button(onClick = { save(selected) }, enabled = !busy) {
                        Text(stringResource(R.string.save_document))
                    }
                    HorizontalDivider()
                }
            }
            Text(stringResource(R.string.attach_to), style = MaterialTheme.typography.bodySmall)
            Box {
                OutlinedButton(onClick = { choosing = true }) {
                    Text(attached?.let(::pointLabel) ?: stringResource(R.string.whole_trip))
                }
                DropdownMenu(expanded = choosing, onDismissRequest = { choosing = false }) {
                    DropdownMenuItem(
                        text = { Text(stringResource(R.string.whole_trip)) },
                        onClick = {
                            choosing = false
                            attach(null)
                        },
                    )
                    trip.points.forEach { point ->
                        DropdownMenuItem(
                            text = { Text(pointLabel(point)) },
                            onClick = {
                                choosing = false
                                attach(point.pointId)
                            },
                        )
                    }
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = pickFile, enabled = !busy) {
                    Text(stringResource(R.string.upload_file))
                }
                Button(onClick = pickMedia, enabled = !busy) {
                    Text(stringResource(R.string.upload_media))
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = capturePhoto, enabled = !busy) {
                    Text(stringResource(R.string.capture_photo))
                }
                Button(onClick = captureVideo, enabled = !busy) {
                    Text(stringResource(R.string.capture_video))
                }
            }
        }
    }
}

@Composable
private fun PointCard(
    point: DriverPoint,
    busy: Boolean,
    navigate: () -> Unit,
    complete: () -> Unit,
    moveUp: () -> Unit,
    moveDown: () -> Unit,
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(
                pointLabel(point),
                style = MaterialTheme.typography.titleMedium,
            )
            Text(if (point.completed) stringResource(R.string.completed) else stringResource(R.string.open_point))
            point.navigation?.let { navigation ->
                Text(
                    navigation.address
                        ?: "${navigation.latitude}, ${navigation.longitude}",
                    style = MaterialTheme.typography.bodySmall,
                )
            }
            if (!point.completed) {
                if (point.navigation != null) {
                    Button(onClick = navigate, modifier = Modifier.fillMaxWidth()) {
                        Text(stringResource(R.string.navigate))
                    }
                }
                Button(onClick = complete, enabled = !busy, modifier = Modifier.fillMaxWidth()) {
                    Text(stringResource(R.string.done_button))
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedButton(onClick = moveUp, enabled = !busy) { Text("↑") }
                    OutlinedButton(onClick = moveDown, enabled = !busy) { Text("↓") }
                }
            }
        }
    }
}
