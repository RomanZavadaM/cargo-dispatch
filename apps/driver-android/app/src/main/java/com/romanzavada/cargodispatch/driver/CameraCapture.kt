package com.romanzavada.cargodispatch.driver

import android.content.Context
import android.net.Uri
import androidx.core.content.FileProvider
import java.io.File
import java.io.IOException
import java.util.UUID

enum class CaptureKind(
    val extension: String,
    val mimeType: String,
) {
    Photo(".jpg", "image/jpeg"),
    Video(".mp4", "video/mp4"),
}

data class PendingCapture(
    val uri: Uri,
    val kind: CaptureKind,
)

fun createCapture(context: Context, kind: CaptureKind): PendingCapture {
    val directory = File(context.cacheDir, "captures").apply { mkdirs() }
    val file = File(directory, "driver-${UUID.randomUUID()}${kind.extension}")
    val uri = FileProvider.getUriForFile(
        context,
        "${context.packageName}.files",
        file,
    )
    return PendingCapture(uri = uri, kind = kind)
}

fun PendingCapture.readUpload(context: Context): PickedUpload {
    val resolver = context.contentResolver
    return PickedUpload(
        name = "driver-capture-${System.currentTimeMillis()}${kind.extension}",
        mimeType = kind.mimeType,
    ) {
        resolver.openInputStream(uri) ?: throw IOException("Captured media cannot be read")
    }
}
