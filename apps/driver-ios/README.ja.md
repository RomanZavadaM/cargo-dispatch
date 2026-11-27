# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · **🇯🇵 日本語** · [🇨🇳 简体中文](README.zh.md)

ネイティブ Apple ドライバークライアントです。1 つの SwiftUI target が iPhone/iPad をサポートし、Driver Browser/Android と **同じ Control API** を使用します。

現在の範囲: Swift 6 + SwiftUI + URLSession、iOS/iPadOS 17+、Driver URL、trip snapshot、complete/reorder、現在の road geometry・地点・最新位置を表示する native MapKit、CoreLocation foreground one-shot 位置、ファイル/写真/動画転送と system camera、位置とメディア用の durable retry queue、`UIBackgroundModes=location` と 25 m filter を使う明示的 Start/Stop background CoreLocation tracking、token を同期しない Keychain 項目（`AfterFirstUnlockThisDeviceOnly`）に保存し非機密の base URL のみ UserDefaults に残す安全な session recovery、reconnect 成功後の旧 raw Driver URL 自動移行、uk/en/de/fr/es/ko/ja/zh-Hans、Swift protocol test と実 synthetic Control trip に接続する Simulator smoke。

ローカル HTTP は Simulator/LAN Debug のみ許可し、Release は HTTPS-first です。

**ドライバーの日常操作**

- 未完了の地点ごとに「ナビで開く」ボタン；
- 「完了にする」は確認を求めます；
- 新しいファイルは地点または運行全体に関連付けられます；
- 画面を開いている間は 15 秒ごとに自動更新し、接続状態を表示します；
- 接続がない間の移動経路（直近 500 件まで）は保存し、順に送信します；
- 配車プログラムが受け付けなかったファイルは残り、再送または削除できます；
- エラーメッセージは端末の言語で、内部用語を使わずに表示します。

今後: 実機 LAN/background tracking 受け入れ、HTTPS/public/NAT/CGNAT フィールド経路、push/notifications、production signing/TestFlight。
