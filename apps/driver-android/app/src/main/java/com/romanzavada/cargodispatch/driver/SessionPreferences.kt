package com.romanzavada.cargodispatch.driver

import android.content.Context

class SessionPreferences(context: Context) {
    private val preferences =
        context.getSharedPreferences("cargo_dispatch_driver_session", Context.MODE_PRIVATE)

    fun load(): SavedDriverSession? {
        val url = preferences.getString(KEY_URL, null)?.trim().orEmpty()
        if (url.isEmpty()) return null
        return SavedDriverSession(
            driverUrl = url,
            emulatorMode = preferences.getBoolean(KEY_EMULATOR, true),
        )
    }

    fun save(driverUrl: String, emulatorMode: Boolean) {
        preferences.edit()
            .putString(KEY_URL, driverUrl.trim())
            .putBoolean(KEY_EMULATOR, emulatorMode)
            .apply()
    }

    fun clear() {
        preferences.edit().clear().apply()
    }

    private companion object {
        const val KEY_URL = "driver_url"
        const val KEY_EMULATOR = "emulator_mode"
    }
}

data class SavedDriverSession(
    val driverUrl: String,
    val emulatorMode: Boolean,
)
