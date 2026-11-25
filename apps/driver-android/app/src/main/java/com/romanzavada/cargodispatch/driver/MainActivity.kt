package com.romanzavada.cargodispatch.driver

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
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
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import com.romanzavada.cargodispatch.driver.model.DriverPoint

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                DriverScreen()
            }
        }
    }
}

@Composable
private fun DriverScreen(viewModel: DriverViewModel = viewModel()) {
    val state by viewModel.state.collectAsStateWithLifecycle()

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
                                    stringResource(R.string.road_geometry_present, it.points.size, it.routing.engine, it.routing.engineVersion)
                                } ?: stringResource(R.string.road_geometry_missing)
                            )
                        }
                    }
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
                Button(onClick = connect, enabled = !state.busy) {
                    Text(if (state.busy) stringResource(R.string.connecting) else stringResource(R.string.connect))
                }
                if (state.trip != null) {
                    Button(onClick = refresh, enabled = !state.busy) {
                        Text(stringResource(R.string.refresh))
                    }
                }
            }
            state.connectedBaseUrl?.let {
                Text(stringResource(R.string.control_prefix, it), style = MaterialTheme.typography.bodySmall)
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
