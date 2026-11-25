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
                Text("Cargo Dispatch — Driver App", style = MaterialTheme.typography.headlineSmall)
                Text(
                    "Native Android client використовує той самий Control API, що й Driver Browser.",
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
                                "Рейс ${trip.tripId.take(8)}…",
                                style = MaterialTheme.typography.titleMedium,
                            )
                            Text("Версія ${trip.tripVersion}; local revision ${trip.localRouteRevision}")
                            Text("Точки: ${trip.points.size}; документи: ${trip.documents.size}")
                            Text(
                                trip.roadGeometry?.let {
                                    "Road geometry: ${it.points.size} точок (${it.routing.engine} ${it.routing.engineVersion})"
                                } ?: "Road geometry: немає / інвалідована"
                            )
                        }
                    }
                }

                item {
                    Text("Маршрут", style = MaterialTheme.typography.titleLarge)
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
            Text("Підключення до Control", style = MaterialTheme.typography.titleMedium)
            OutlinedTextField(
                value = state.driverUrl,
                onValueChange = setDriverUrl,
                modifier = Modifier.fillMaxWidth(),
                label = { Text("Driver URL з Control") },
                placeholder = { Text("http://127.0.0.1:12345/driver/trip-...") },
                singleLine = true,
            )
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Checkbox(
                    checked = state.emulatorMode,
                    onCheckedChange = setEmulatorMode,
                )
                Column {
                    Text("Android Emulator")
                    Text(
                        "127.0.0.1/localhost автоматично → 10.0.2.2",
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = connect, enabled = !state.busy) {
                    Text(if (state.busy) "Підключення…" else "Підключити")
                }
                if (state.trip != null) {
                    Button(onClick = refresh, enabled = !state.busy) {
                        Text("Оновити")
                    }
                }
            }
            state.connectedBaseUrl?.let {
                Text("Control: $it", style = MaterialTheme.typography.bodySmall)
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
            Text(if (point.completed) "Виконано" else "Відкрита точка")
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
                    Button(onClick = complete, enabled = !busy) { Text("Виконано") }
                }
            }
        }
    }
}
