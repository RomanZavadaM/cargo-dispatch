package com.romanzavada.cargodispatch.driver

import android.annotation.SuppressLint
import android.content.Context
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Build
import android.os.Bundle
import android.os.Looper
import com.romanzavada.cargodispatch.driver.model.LocationUpdate

class ForegroundLocationProvider(context: Context) {
    private val manager = context.getSystemService(LocationManager::class.java)

    @SuppressLint("MissingPermission")
    fun current(onResult: (Result<LocationUpdate>) -> Unit) {
        val provider = preferredProvider()
        if (provider == null) {
            onResult(Result.failure(IllegalStateException("No enabled location provider")))
            return
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            manager.getCurrentLocation(provider, null, Runnable::run) { location ->
                onResult(
                    location?.let { Result.success(it.toUpdate()) }
                        ?: Result.failure(IllegalStateException("Location unavailable")),
                )
            }
            return
        }

        @Suppress("DEPRECATION")
        manager.requestSingleUpdate(
            provider,
            object : LocationListener {
                override fun onLocationChanged(location: Location) {
                    onResult(Result.success(location.toUpdate()))
                }

                @Deprecated("Deprecated in Android")
                override fun onStatusChanged(provider: String?, status: Int, extras: Bundle?) = Unit

                override fun onProviderEnabled(provider: String) = Unit

                override fun onProviderDisabled(provider: String) {
                    onResult(Result.failure(IllegalStateException("Location provider disabled")))
                }
            },
            Looper.getMainLooper(),
        )
    }

    private fun preferredProvider(): String? =
        listOf(LocationManager.GPS_PROVIDER, LocationManager.NETWORK_PROVIDER)
            .firstOrNull { provider ->
                runCatching { manager.isProviderEnabled(provider) }.getOrDefault(false)
            }

    private fun Location.toUpdate(): LocationUpdate =
        LocationUpdate(
            latitude = latitude,
            longitude = longitude,
            accuracyM = if (hasAccuracy()) accuracy.toDouble() else null,
            observedAtUnixMs = time.takeIf { it > 0 } ?: System.currentTimeMillis(),
        )
}
