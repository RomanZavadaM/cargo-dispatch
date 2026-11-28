package com.romanzavada.cargodispatch.driver

import android.content.ContentResolver
import android.net.Uri
import android.provider.OpenableColumns
import java.io.IOException
import java.io.InputStream

/** A chosen file. Its content is copied to the queue, never held in memory. */
class PickedUpload(
    val name: String,
    val mimeType: String,
    val open: () -> InputStream,
)

fun ContentResolver.readUpload(uri: Uri): PickedUpload {
    val name =
        query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) else null
        }
            ?.takeIf { it.isNotBlank() }
            ?: "driver-upload"

    val mime = getType(uri).orEmpty().ifBlank { "application/octet-stream" }
    return PickedUpload(name = name, mimeType = mime) {
        openInputStream(uri) ?: throw IOException("Selected file cannot be read")
    }
}
