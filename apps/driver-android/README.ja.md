# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · **🇯🇵 日本語** · [🇨🇳 简体中文](README.zh.md)

ネイティブ Android ドライバークライアントです。**Driver Browser を置き換えず**、別のサーバープロトコルも作りません。同じ Control API と運行ルールを使用します。

現在の範囲: Kotlin + Jetpack Compose、Android 16/API 36（minSdk 26）、Driver URL、active-trip snapshot、complete/reorder、現在の road geometry・地点・最新位置を表示する native MapLibre、foreground one-shot 位置、ファイル/写真/動画転送と system camera、最新位置/メディア用 durable retry queue、15 秒 / 25 m ポリシーの foreground location service による明示的 Start/Stop background tracking、token を Android Keystore の AES/GCM で暗号化し非機密の base URL/Emulator mode のみ private preferences に残す安全な session recovery、reconnect 成功後の旧 raw Driver URL 自動移行、uk/en/de/fr/es/ko/ja/zh-CN ローカライズ、protocol/unit test と実 synthetic Control trip に接続する Emulator smoke。

平文 HTTP はローカル debug のみで許可し、公開経路は HTTPS-first です。

今後: 実機 LAN/background tracking 受け入れ、HTTPS/public/NAT/CGNAT フィールド経路、push/notifications、production signing。
