# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · **🇯🇵 日本語** · [🇨🇳 简体中文](README.zh.md)

ネイティブ Apple ドライバークライアントです。1 つの SwiftUI target が iPhone/iPad をサポートし、Driver Browser/Android と **同じ Control API** を使用します。

現在の範囲: Swift 6 + SwiftUI + URLSession、iOS/iPadOS 17+、Driver URL、trip snapshot、complete/reorder、permanent point ID + local revision、optional road geometry、CoreLocation foreground one-shot 位置送信、Control→iPhone/iPad 書類ダウンロード/fileExporter 保存、fileImporter + PhotosPicker による iPhone/iPad→Control ファイル/写真/動画アップロード、最後の Driver URL 復元、位置権限メッセージのローカライズ、uk/en/de/fr/es/ko/ja/zh-Hans、Swift protocol test と Simulator build/install/launch/screenshot CI。

ローカル HTTP は Simulator/LAN Debug のみ許可し、Release は HTTPS-first です。

今後: native MapKit map、カメラ撮影、background GPS、実際の synthetic Control trip との connected smoke、offline queue/retry、Keychain 強化、push、production signing/TestFlight。
