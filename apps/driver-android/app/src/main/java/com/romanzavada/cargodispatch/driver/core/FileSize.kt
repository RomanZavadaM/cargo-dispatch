package com.romanzavada.cargodispatch.driver.core

import java.util.Locale

enum class SizeUnit { Bytes, Kilobytes, Megabytes }

data class FileSize(val value: String, val unit: SizeUnit) {
    companion object {
        fun of(bytes: Long, locale: Locale = Locale.getDefault()): FileSize = when {
            bytes < 1024 -> FileSize(bytes.toString(), SizeUnit.Bytes)
            bytes < 1024 * 1024 -> FileSize(
                String.format(locale, "%.1f", bytes / 1024.0),
                SizeUnit.Kilobytes,
            )
            else -> FileSize(
                String.format(locale, "%.1f", bytes / (1024.0 * 1024.0)),
                SizeUnit.Megabytes,
            )
        }
    }
}
