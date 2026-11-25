# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · **🇬🇧 English** · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Native Apple driver client. One SwiftUI target supports iPhone and iPad and uses the **same Control API** as Driver Browser and Android Driver App.

Current scope: Swift 6 + SwiftUI + URLSession; iOS/iPadOS 17+; Driver URL; trip snapshot; complete/reorder; permanent point IDs + local route revision; optional road geometry; one-shot CoreLocation foreground GPS to Control; Control→iPhone/iPad document download through native fileExporter; iPhone/iPad→Control file/photo/video through fileImporter + PhotosPicker; restart recovery of the last Driver URL; localized location permission; uk/en/de/fr/es/ko/ja/zh-Hans localization; pure Swift protocol tests and Simulator build/install/launch/screenshot CI.

Local HTTP is allowed only for Simulator/LAN Debug; Release remains HTTPS-first.

Still ahead: native MapKit map, camera capture, background GPS, connected Simulator smoke against a real synthetic Control trip, offline queue/retry, Keychain hardening, push/notifications, and production signing/TestFlight.
