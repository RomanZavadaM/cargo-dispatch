# Cargo Dispatch Driver — iOS/iPadOS スキャフォールド

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · **🇯🇵 日本語** · [🇨🇳 简体中文](README.zh.md)

最初のネイティブ Apple スライスです。1 つの SwiftUI app target が iPhone/iPad をサポートし、Driver Browser と Android Driver App と **同じ Control API** を使用します。

現在の範囲: Swift 6 + SwiftUI + URLSession、iOS/iPadOS 17+、Driver URL、trip snapshot、complete/reorder、永久 point ID + local revision、optional route geometry、pure Swift core test、uk/en/de/fr/es/ko/ja/zh-Hans ローカライズ。

ローカル HTTP は Simulator/LAN の Debug のみ許可し、Release は HTTPS-first です。

未実装: ネイティブ地図、カメラ/写真/動画、文書表示/共有、バックグラウンド GPS、オフラインキュー、安全なセッション保存、push、production signing/TestFlight。
