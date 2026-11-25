# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · **🇬🇧 English** · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Native Android driver client. It **does not replace Driver Browser** and does not create a separate server protocol: it uses the same Control API and trip rules.

Current scope: Kotlin + Jetpack Compose; Android 16/API 36 with minSdk 26; Driver URL connection; active-trip snapshot; complete/reorder; native MapLibre map with current road geometry, markers and latest position; one-shot foreground location; file/photo/video transfer and system camera; durable retry queue for newest location/media; explicit Start/Stop background tracking through a foreground location service with a 15 s / 25 m policy; secure session recovery where the token is AES/GCM encrypted by Android Keystore while only non-secret base URL/emulator mode remain in private preferences; automatic migration away from the legacy raw Driver URL after a successful reconnect; uk/en/de/fr/es/ko/ja/zh-CN localization; protocol/unit tests and connected Emulator smoke against a real synthetic Control trip.

Debug builds allow cleartext HTTP only for local testing. Public/production traffic is HTTPS-first.

Still ahead: real-device LAN/background-tracking acceptance, HTTPS/public/NAT/CGNAT field acceptance, push/notifications, and production signing.
