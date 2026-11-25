# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · **🇩🇪 Deutsch** · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Nativer Apple-Fahrerclient. Ein SwiftUI-Target unterstützt iPhone und iPad und verwendet **dieselbe Control-API** wie Driver Browser und Android Driver App.

Aktueller Umfang: Swift 6 + SwiftUI + URLSession; iOS/iPadOS 17+; Driver-URL; Trip-Snapshot; Complete/Reorder; permanente Punkt-IDs + lokale Routenrevision; optionale Routengeometrie; einmalige CoreLocation-Vordergrundposition an Control; Dokumentdownload Control→iPhone/iPad über fileExporter; Datei/Foto/Video iPhone/iPad→Control über fileImporter + PhotosPicker; Wiederherstellung der letzten Driver-URL; lokalisierter Standort-Hinweis; uk/en/de/fr/es/ko/ja/zh-Hans; Swift-Protokolltests und Simulator-Build/Install/Launch/Screenshot-CI.

Lokales HTTP ist nur in Debug für Simulator/LAN erlaubt; Release bleibt HTTPS-first.

Noch offen: native MapKit-Karte, Kameraaufnahme, Hintergrund-GPS, verbundener Simulator-Smoke gegen einen echten synthetischen Control-Trip, Offline-Queue/Retry, Keychain-Härtung, Push und Production Signing/TestFlight.
