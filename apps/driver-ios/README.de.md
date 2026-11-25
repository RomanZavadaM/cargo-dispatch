# Cargo Dispatch Driver — iOS/iPadOS-Gerüst

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · **🇩🇪 Deutsch** · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Erster nativer Apple-Slice. Ein SwiftUI-App-Target unterstützt iPhone und iPad und verwendet **dieselbe Control-API** wie Driver Browser und Android Driver App.

Aktueller Umfang: Swift 6 + SwiftUI + URLSession, iOS/iPadOS 17+, Driver-URL, Trip-Snapshot, Complete/Reorder, permanente Punkt-IDs + lokale Routenrevision, optionales Routengeometrie-Modell, reine Swift-Core-Tests und Lokalisierungen uk/en/de/fr/es/ko/ja/zh-Hans.

Lokales HTTP ist nur im Debug für Simulator/LAN erlaubt; Release bleibt HTTPS-first.

Noch nicht umgesetzt: native Karte, Kamera/Foto/Video, Dokument öffnen/teilen, Hintergrund-GPS, Offline-Warteschlange, sichere persistente Session, Push, Production Signing/TestFlight.
