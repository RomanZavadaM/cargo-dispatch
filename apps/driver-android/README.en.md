# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · **🇬🇧 English** · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Native Android driver client. It **does not replace Driver Browser** and does not create a separate server protocol: it uses the same Control API and trip rules.

Current scope: Kotlin + Jetpack Compose; Android 16/API 36 with minSdk 26; Driver URL connection; Emulator host loopback mapping to `10.0.2.2`; active-trip snapshot; complete/reorder; optional road geometry; one-shot foreground location to Control; Control→Android document download with native save; Android→Control file/photo/video upload through system pickers; restart recovery of the last Driver URL/emulator mode; uk/en/de/fr/es/ko/ja/zh-CN localization; protocol/unit tests and Emulator install/launch/screenshot CI.

Debug builds allow cleartext HTTP only for local testing. Public/production traffic is HTTPS-first.

Still ahead: native map, camera capture, background GPS, connected Emulator smoke against a real synthetic Control trip, offline queue/retry, secure Keystore hardening, push/notifications, and production signing.
