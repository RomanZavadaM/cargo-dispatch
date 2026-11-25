package com.romanzavada.cargodispatch.driver

import android.content.ContentResolver
import android.net.Uri
import android.provider.OpenableColumns

data class PickedUpload(
    val name: String,
    val mimeType: String,
    val bytes: ByteArray,
)

fun ContentResolver.readUpload(uri: Uri): PickedUpload {
    val name =
        query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) else null
        }
            ?.takeIf { it.isNotBlank() }
            ?: "driver-upload"

    val mime = getType(uri).orEmpty().ifBlank { "application/octet-stream" }
    val bytes = openInputStream(uri)?.use { it.readBytes() }
        ?: error("Selected file cannot be read")

    return PickedUpload(name = name, mimeType = mime, bytes = bytes)
}
