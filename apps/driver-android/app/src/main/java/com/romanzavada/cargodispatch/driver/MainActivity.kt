package com.romanzavada.cargodispatch.driver

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.romanzavada.cargodispatch.driver.model.DriverDocument
import com.romanzavada.cargodispatch.driver.model.DriverPoint
import org.maplibre.android.MapLibre

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
                    emulatorMode = intent.getBooleanExtra(EXTRA_EMULATOR_MODE, true),
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

    val saveDocument = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("*/*"),
    ) { uri ->
        val selected = pendingSave
        pendingSave = null
        if (uri != null && selected != null) {
            viewModel.download(selected.documentId) { result ->
                result.onSuccess { bytes ->
                    runCatching {
                        context.contentResolver.openOutputStream(uri, "w")?.use { output ->
                            output.write(bytes)
                        } ?: error("Cannot open destination")
                    }
                }
            }
        }
    }

    val filePicker = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri != null) {
            runCatching { context.contentResolver.readUpload(uri) }
                .onSuccess(viewModel::upload)
        }
    }

    val mediaPicker = rememberLauncherForActivityResult(
        ActivityResultContracts.PickVisualMedia(),
    ) { uri ->
        if (uri != null) {
            runCatching { context.contentResolver.readUpload(uri) }
                .onSuccess(viewModel::upload)
        }
    }

    val takePhoto = rememberLauncherForActivityResult(
        ActivityResultContracts.TakePicture(),
    ) { success ->
        val capture = pendingCapture
        pendingCapture = null
        if (success && capture != null) {
            runCatching { capture.readUpload(context) }
                .onSuccess(viewModel::upload)
        }
    }

    val captureVideo = rememberLauncherForActivityResult(
        ActivityResultContracts.CaptureVideo(),
    ) { success ->
        val capture = pendingCapture
        pendingCapture = null
        if (success && capture != null) {
            runCatching { capture.readUpload(context) }
                .onSuccess(viewModel::upload)
        }
    }

    val locationPermission = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        if (granted) {
            locationProvider.current { result ->
                result.onSuccess(viewModel::sendLocation)
            }
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
            viewModel.reportError(context.getString(R.string.tracking_permission_required))
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

    val stopTracking = {
        context.stopService(Intent(context, DriverTrackingService::class.java))
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

            state.error?.let { problem ->
                item {
                    Text(problem, color = MaterialTheme.colorScheme.error)
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
                            Text(stringResource(R.string.version_revision, trip.tripVersion, trip.localRouteRevision))
                            Text(stringResource(R.string.points_documents, trip.points.size, trip.documents.size))
                            Text(
                                trip.roadGeometry?.let {
                                    stringResource(
                                        R.string.road_geometry_present,
                                        it.points.size,
                                        it.routing.engine,
                                        it.routing.engineVersion,
                                    )
                                } ?: stringResource(R.string.road_geometry_missing)
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
                        documents = trip.documents,
                        busy = state.busy,
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
                        complete = { viewModel.complete(point.pointId) },
                        moveUp = { viewModel.moveOpen(point.pointId, -1) },
                        moveDown = { viewModel.moveOpen(point.pointId, 1) },
                    )
                }
            }
        }
    }
}

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
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = connect, enabled = !state.busy && state.driverUrl.isNotBlank()) {
                    Text(if (state.busy) stringResource(R.string.connecting) else stringResource(R.string.connect))
                }
                if (state.trip != null) {
                    Button(onClick = refresh, enabled = !state.busy) {
                        Text(stringResource(R.string.refresh))
                    }
                    Button(onClick = disconnect, enabled = !state.busy) {
                        Text(stringResource(R.string.disconnect))
                    }
                }
            }
            state.connectedBaseUrl?.let {
                Text(stringResource(R.string.control_prefix, it), style = MaterialTheme.typography.bodySmall)
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
                        location.latitude,
                        location.longitude,
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
    documents: List<DriverDocument>,
    busy: Boolean,
    pickFile: () -> Unit,
    pickMedia: () -> Unit,
    capturePhoto: () -> Unit,
    captureVideo: () -> Unit,
    save: (DriverDocument) -> Unit,
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(stringResource(R.string.documents_title), style = MaterialTheme.typography.titleMedium)
            if (documents.isEmpty()) {
                Text(stringResource(R.string.documents_empty))
            } else {
                documents.forEach { selected ->
                    Text(
                        "${selected.originalName} · ${selected.size} B",
                        style = MaterialTheme.typography.bodySmall,
                    )
                    Button(onClick = { save(selected) }, enabled = !busy) {
                        Text(stringResource(R.string.save_document))
                    }
                    HorizontalDivider()
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
                "${point.sequence}. ${point.title}",
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
                HorizontalDivider()
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(onClick = moveUp, enabled = !busy) { Text("↑") }
                    Button(onClick = moveDown, enabled = !busy) { Text("↓") }
                    Button(onClick = complete, enabled = !busy) { Text(stringResource(R.string.done_button)) }
                }
            }
        }
    }
}
