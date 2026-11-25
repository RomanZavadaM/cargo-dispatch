# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · **🇩🇪 Deutsch** · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Nativer Android-Fahrerclient. Er **ersetzt den Driver Browser nicht** und führt kein separates Serverprotokoll ein: Er verwendet dieselbe Control-API und dieselben Tourregeln.

Aktueller Umfang: Kotlin + Jetpack Compose; Android 16/API 36, minSdk 26; Driver-URL; Emulator-Loopback über `10.0.2.2`; Active-Trip-Snapshot; Complete/Reorder; optionale Routengeometrie; einmalige Vordergrund-Position an Control; Dokumentdownload Control→Android mit nativem Speichern; Datei/Foto/Video Android→Control über System-Picker; Wiederherstellung der letzten Driver-URL/des Emulator-Modus; Lokalisierung uk/en/de/fr/es/ko/ja/zh-CN; Protokoll-/Unit-Tests und Emulator-Install/Launch/Screenshot-CI.

Unverschlüsseltes HTTP ist nur im Debug-Build für lokale Tests erlaubt. Der öffentliche Produktpfad ist HTTPS-first.

Noch offen: native Karte, Kameraaufnahme, Hintergrund-GPS, verbundener Emulator-Smoke gegen einen echten synthetischen Control-Trip, Offline-Queue/Retry, Keystore-Härtung, Push und Production Signing.
