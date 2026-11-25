# Cargo Dispatch Driver — Android スキャフォールド

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · **🇯🇵 日本語** · [🇨🇳 简体中文](README.zh.md)

最初のネイティブ Android スライスです。**Driver Browser を置き換えず**、別のサーバープロトコルも作りません。同じ Control API を使用します。

現在の範囲: Kotlin + Jetpack Compose、Android 16/API 36、Driver URL 接続、active trip snapshot、complete/reorder、route geometry モデル、reconnect/error 状態、protocol/unit test、uk/en/de/fr/es/ko/ja/zh-CN ローカライズ。

Android Emulator ではホストの `127.0.0.1`/localhost を `10.0.2.2` に変換します。平文 HTTP はローカル debug のみで許可し、公開経路は HTTPS を使用します。

未実装: カメラ/写真/動画、書類の表示/共有、バックグラウンド GPS、オフラインキュー、安全なセッション保存、push、production signing。
