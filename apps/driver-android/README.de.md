# Cargo Dispatch Driver — Android-Gerüst

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · **🇩🇪 Deutsch** · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Erster nativer Android-Slice. Er **ersetzt den Driver Browser nicht** und führt kein zweites Serverprotokoll ein: verwendet wird dieselbe Control-API.

Aktueller Umfang: Kotlin + Jetpack Compose, Android 16/API 36, Verbindung über Driver-URL, Active-Trip-Snapshot, Complete/Reorder, Modell für Routengeometrie, Reconnect/Fehlerstatus, Protokoll-/Unit-Tests sowie Lokalisierungen uk/en/de/fr/es/ko/ja/zh-CN.

Im Android Emulator wird Host-`127.0.0.1`/localhost auf `10.0.2.2` abgebildet. Debug-Builds erlauben lokales HTTP nur für Entwicklung; der öffentliche Produktpfad soll HTTPS verwenden.

Noch nicht umgesetzt: Kamera/Foto/Video, Dokument öffnen/teilen, Hintergrund-GPS, Offline-Warteschlange, sichere persistente Session, Push, Production Signing.
