# Cargo Dispatch Driver — iOS/iPadOS scaffold

[🇺🇦 Українська](README.md) · **🇬🇧 English** · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

First native Apple slice. One SwiftUI app target supports iPhone and iPad and uses the **same Control API** as Driver Browser and Android Driver App.

Current scope: Swift 6 + SwiftUI + URLSession, iOS/iPadOS 17+, Driver URL connection, trip snapshot, complete/reorder, permanent point IDs + local route revision, optional road geometry model, pure Swift core tests, and localization in uk/en/de/fr/es/ko/ja/zh-Hans.

Debug local HTTP is allowed only for Simulator/LAN development; Release remains HTTPS-first.

Not yet implemented: native map rendering, camera/photo/video, document open/share, background GPS, offline queue, secure persisted session, push, production signing/TestFlight.
