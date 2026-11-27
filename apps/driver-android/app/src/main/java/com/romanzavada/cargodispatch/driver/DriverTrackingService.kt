package com.romanzavada.cargodispatch.driver

import android.Manifest
import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Build
import android.os.IBinder
import android.os.Looper
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import com.romanzavada.cargodispatch.driver.core.SessionLink
import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import com.romanzavada.cargodispatch.driver.net.ControlApi
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

class DriverTrackingService : Service() {
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private lateinit var locationManager: LocationManager
    private lateinit var notificationManager: NotificationManager
    private var listener: LocationListener? = null
    private var api: ControlApi? = null
    private var queue: PendingOutboundQueue? = null

    override fun onCreate() {
        super.onCreate()
        locationManager = getSystemService(LocationManager::class.java)
        notificationManager = getSystemService(NotificationManager::class.java)
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            stopSelf()
            return START_NOT_STICKY
        }

        startForeground(NOTIFICATION_ID, notification(pending = null))

        val session = SessionPreferences(applicationContext).load()
        if (session == null) {
            stopSelf()
            return START_NOT_STICKY
        }

        val target = runCatching {
            SessionLink.parseDriverUrl(session.driverUrl, session.emulatorMode)
        }.getOrElse {
            stopSelf()
            return START_NOT_STICKY
        }

        api = ControlApi(target.baseUrl, target.token)
        queue = PendingOutboundQueue.forSession(
            File(filesDir, "outbound-queue"),
            target.baseUrl,
            target.token,
        )

        if (!hasLocationPermission()) {
            stopSelf()
            return START_NOT_STICKY
        }

        DriverTrackingState.setActive(true)
        startLocationUpdates()
        serviceScope.launch {
            flushPending()
        }
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        listener?.let(locationManager::removeUpdates)
        listener = null
        DriverTrackingState.setActive(false)
        serviceScope.cancel()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    @SuppressLint("MissingPermission")
    private fun startLocationUpdates() {
        if (listener != null) return

        val provider = listOf(
            LocationManager.GPS_PROVIDER,
            LocationManager.NETWORK_PROVIDER,
        ).firstOrNull { candidate ->
            runCatching { locationManager.isProviderEnabled(candidate) }.getOrDefault(false)
        } ?: run {
            stopSelf()
            return
        }

        val created = LocationListener { location ->
            serviceScope.launch {
                handleLocation(location)
            }
        }
        listener = created
        locationManager.requestLocationUpdates(
            provider,
            DriverTrackingPolicy.MIN_TIME_MS,
            DriverTrackingPolicy.MIN_DISTANCE_M,
            created,
            Looper.getMainLooper(),
        )
    }

    private suspend fun handleLocation(location: Location) {
        val update = LocationUpdate(
            latitude = location.latitude,
            longitude = location.longitude,
            accuracyM = if (location.hasAccuracy()) location.accuracy.toDouble() else null,
            observedAtUnixMs = location.time.takeIf { it > 0 } ?: System.currentTimeMillis(),
        )

        val pending = queue ?: return

        pending.enqueueLocation(update)
        flushPending()
    }

    private suspend fun flushPending() {
        val pending = queue ?: return
        val client = api ?: return
        try {
            OutboundSender.sendLocations(pending, client::sendLocation)
        } finally {
            updateNotification()
        }
    }

    private suspend fun updateNotification() {
        val count = queue?.pendingCount() ?: 0
        notificationManager.notify(NOTIFICATION_ID, notification(count))
    }

    private fun notification(pending: Int?): Notification {
        val stopIntent = Intent(this, DriverTrackingService::class.java).apply {
            action = ACTION_STOP
        }
        val stopPendingIntent = PendingIntent.getService(
            this,
            0,
            stopIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val text = if (pending != null && pending > 0) {
            getString(R.string.tracking_notification_pending, pending)
        } else {
            getString(R.string.tracking_notification_active)
        }

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_menu_mylocation)
            .setContentTitle(getString(R.string.tracking_notification_title))
            .setContentText(text)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .addAction(
                android.R.drawable.ic_media_pause,
                getString(R.string.tracking_stop),
                stopPendingIntent,
            )
            .build()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        notificationManager.createNotificationChannel(
            NotificationChannel(
                CHANNEL_ID,
                getString(R.string.tracking_channel_name),
                NotificationManager.IMPORTANCE_LOW,
            ).apply {
                description = getString(R.string.tracking_channel_description)
            },
        )
    }

    private fun hasLocationPermission(): Boolean =
        ContextCompat.checkSelfPermission(
            this,
            Manifest.permission.ACCESS_FINE_LOCATION,
        ) == PackageManager.PERMISSION_GRANTED ||
            ContextCompat.checkSelfPermission(
                this,
                Manifest.permission.ACCESS_COARSE_LOCATION,
            ) == PackageManager.PERMISSION_GRANTED

    companion object {
        const val ACTION_STOP = "com.romanzavada.cargodispatch.driver.STOP_TRACKING"
        private const val CHANNEL_ID = "driver-tracking"
        private const val NOTIFICATION_ID = 4107
    }
}
