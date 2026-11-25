# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · **🇯🇵 日本語** · [🇨🇳 简体中文](README.zh.md)

ネイティブ Android ドライバークライアントです。**Driver Browser を置き換えず**、別のサーバープロトコルも作りません。同じ Control API と運行ルールを使用します。

現在の範囲: Kotlin + Jetpack Compose、Android 16/API 36（minSdk 26）、Driver URL、Emulator の host loopback `10.0.2.2`、active-trip snapshot、complete/reorder、optional road geometry、foreground one-shot 位置送信、Control→Android 書類ダウンロードとネイティブ保存、system picker による Android→Control ファイル/写真/動画アップロード、最後の Driver URL/Emulator mode 復元、uk/en/de/fr/es/ko/ja/zh-CN ローカライズ、protocol/unit test と Emulator install/launch/screenshot CI。

平文 HTTP はローカル debug のみで許可し、公開経路は HTTPS-first です。

今後: native map、カメラ撮影、background GPS、実際の synthetic Control trip との connected smoke、offline queue/retry、Keystore 強化、push、production signing。
