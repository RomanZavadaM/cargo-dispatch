package com.romanzavada.cargodispatch.driver

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import com.romanzavada.cargodispatch.driver.model.DriverTrip
import org.maplibre.android.camera.CameraUpdateFactory
import org.maplibre.android.geometry.LatLng
import org.maplibre.android.geometry.LatLngBounds
import org.maplibre.android.maps.MapView
import org.maplibre.android.maps.Style

@Composable
fun DriverMapCard(trip: DriverTrip) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val styleJson = remember(trip) { DriverMapStyle.styleJson(trip) }
    val extent = remember(trip) { DriverMapStyle.extent(trip) }
    val mapView = remember {
        MapView(context).apply {
            onCreate(null)
        }
    }

    DisposableEffect(lifecycleOwner, mapView) {
        val observer = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_START -> mapView.onStart()
                Lifecycle.Event.ON_RESUME -> mapView.onResume()
                Lifecycle.Event.ON_PAUSE -> mapView.onPause()
                Lifecycle.Event.ON_STOP -> mapView.onStop()
                else -> Unit
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        if (lifecycleOwner.lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED)) {
            mapView.onStart()
        }
        if (lifecycleOwner.lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)) {
            mapView.onResume()
        }
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
            if (lifecycleOwner.lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)) {
                mapView.onPause()
            }
            if (lifecycleOwner.lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED)) {
                mapView.onStop()
            }
            mapView.onDestroy()
        }
    }

    Card(modifier = Modifier.fillMaxWidth()) {
        Column(Modifier.padding(14.dp)) {
            Text(
                stringResource(R.string.map_title),
                style = MaterialTheme.typography.titleMedium,
            )
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(300.dp),
            ) {
                AndroidView(
                    modifier = Modifier.fillMaxWidth().height(300.dp),
                    factory = { mapView },
                    update = { view ->
                        view.getMapAsync { map ->
                            map.uiSettings.isAttributionEnabled = true
                            map.setStyle(Style.Builder().fromJson(styleJson)) {
                                when {
                                    extent.size >= 2 -> {
                                        val bounds = LatLngBounds.Builder()
                                            .includes(extent.map { LatLng(it.latitude, it.longitude) })
                                            .build()
                                        map.moveCamera(
                                            CameraUpdateFactory.newLatLngBounds(bounds, 56),
                                        )
                                    }
                                    extent.size == 1 -> {
                                        val point = extent.first()
                                        map.moveCamera(
                                            CameraUpdateFactory.newLatLngZoom(
                                                LatLng(point.latitude, point.longitude),
                                                14.0,
                                            ),
                                        )
                                    }
                                    else -> map.moveCamera(
                                        CameraUpdateFactory.newLatLngZoom(LatLng(49.0, 31.0), 5.0),
                                    )
                                }
                            }
                        }
                    },
                )
            }
            Text(
                stringResource(R.string.map_provider_note),
                style = MaterialTheme.typography.bodySmall,
            )
        }
    }
}
