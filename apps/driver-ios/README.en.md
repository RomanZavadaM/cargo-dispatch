# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · **🇬🇧 English** · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Native Apple driver client. One SwiftUI target supports iPhone and iPad and uses the **same Control API** as Driver Browser and Android Driver App.

Current scope: Swift 6 + SwiftUI + URLSession; iOS/iPadOS 17+; Driver URL; trip snapshot; complete/reorder; native MapKit map with current road geometry, markers and latest position; one-shot foreground CoreLocation; file/photo/video transfer and system camera; durable retry queue for newest location/media; explicit Start/Stop background CoreLocation tracking with `UIBackgroundModes=location` and a 25 m filter; secure session recovery with the token in a non-synchronizing Keychain item (`AfterFirstUnlockThisDeviceOnly`) and only the non-secret base URL in UserDefaults; automatic migration away from the legacy raw Driver URL after a successful reconnect; uk/en/de/fr/es/ko/ja/zh-Hans localization; pure Swift protocol tests and connected Simulator smoke against a real synthetic Control trip.

Local HTTP is allowed only for Simulator/LAN Debug; Release remains HTTPS-first.

Still ahead: real-device LAN/background-tracking acceptance, HTTPS/public/NAT/CGNAT field acceptance, push/notifications, and production signing/TestFlight.
