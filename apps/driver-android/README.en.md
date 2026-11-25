# Cargo Dispatch Driver — Android scaffold

[🇺🇦 Українська](README.md) · **🇬🇧 English** · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

The first native Android slice. It **does not replace Driver Browser** and does not introduce a second server protocol: it uses the same Control API.

Current scope: Kotlin + Jetpack Compose, Android 16/API 36, Driver URL connection, active-trip snapshot, complete/reorder, route geometry model, reconnect/error state, protocol/unit tests, and localization in uk/en/de/fr/es/ko/ja/zh-CN.

For Android Emulator, host `127.0.0.1`/localhost is mapped to `10.0.2.2`. Debug builds allow local cleartext HTTP only for development; production/public traffic is expected to use HTTPS.

Not yet implemented: camera/photo/video, document open/share, background GPS, offline queue, secure persisted session, push, production signing.
