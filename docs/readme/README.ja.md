# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · **🇯🇵 日本語** · [🇨🇳 简体中文](README.zh.md)

> **ステータス:** 開発中です。現在の安定版は **v0.1.0** です。8言語の完全なローカライズを統合済みで、次の大きな作業はルート Planner です。

## 製品について

**cargo-dispatch** は貨物車両群向けの配車システムです。貨物の選定、運行の作成・再計画、実行への引き渡し、そして配車担当者による実際の進行管理を目的としています。

主な開発言語は **Rust** です。

## 現在実装されている機能

- Tauri/Leptos ベースの独立した desktop **Control** アプリ;
- アクティブな運行の永続ローカル保存と再起動後の復旧;
- Planner と Control 間のバージョン付き trip package;
- 電話・タブレット・ノート PC・desktop browser 向けの単一の適応型 **Driver Browser**;
- Control と Driver Browser の両方から開いている地点の順序変更;
- PDF/通常ファイル **Control → Driver Browser**;
- ファイル・写真・動画 **Driver Browser → Control** とローカル保存;
- 同一運行の 2 つの同時 browser client と、別 browser DB を使わない reconnect;
- Control と Driver Browser のインタラクティブ OpenStreetMap;
- テストとローカル運用用の loopback および明示的 LAN モード.

## ネイティブ Driver App

2 つのネイティブクライアントはすでに `main` に統合されています。

- **Android** — Jetpack Compose、route/complete/reorder、ファイル/写真/動画、foreground GPS、session recovery、native MapLibre + OpenStreetMap、現在の road geometry、地点/最新位置、システム写真・動画撮影；Emulator build/install/launch/screenshot PASS。
- **iPhone/iPad** — SwiftUI、route/complete/reorder、ファイル/写真/動画、foreground CoreLocation、session recovery、native MapKit、現在の geometry、地点/最新位置、システム写真・動画撮影；Simulator build/install/launch/screenshot PASS。

両方とも同じ Control API を使用し、別の運行データベースは作りません。

r4/r5 以降、両方の native client は位置情報とメディア用の durable retry queue も備えています。Android r5 は foreground location service、常時表示の Stop 通知、15 秒 / 25 m のポリシーによる明示的な Start/Stop バックグラウンド運行追跡を追加します。Apple r5 は明示的な Start/Stop CoreLocation バックグラウンド追跡、`UIBackgroundModes=location`、25 m フィルタを追加し、別の必要性がない限り `Always` 権限を要求しません。

## Driver Browser

Driver Browser は電話版と desktop 版に分かれた製品ではなく、**1 つの製品**です。標準レイアウトは次の 3 つです。

- narrow phone;
- tablet;
- wide desktop/laptop.

データ、経路操作、ファイル、backend は共通で、画面サイズに応じて配置だけが変わります。

## 地図

Control と Driver Browser には、運行地点と最後に受理されたドライバー位置を表示する実際のインタラクティブなベースマップがあります。

`main` には現在、バージョン付き道路 geometry パイプライン（schema 1.3）が統合されています。Planner は派生 geometry と実際の車両 routing profile を一緒に渡し、schema 1.0–1.2 のパッケージも引き続き読み込めます。Control は geometry を保存し、地点順の変更後は設定された runtime Valhalla を使って道路線を再計算します。遅れて届いた routing 応答が、より新しい運行状態を上書きすることはありません。

Valhalla アドレス、地図データセット ID、timeout は Desktop Control で直接設定し、`/status` で利用可否を確認するか routing を明示的に無効化できます。Valhalla が未設定または一時的に利用できない場合でも、新しい地点順は Control に保存され、派生 geometry だけが存在しない状態になります。

## Stable v0.1.0

[Stable v0.1.0 を開く](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0)。

Stable は 1 つの正確なソースから再ビルドされ、次を公開します。
- Windows Control x64 ZIP;
- macOS Control universal ZIP — Intel + Apple Silicon;
- Linux Control x64 tar.gz;
- テストインストール用 Android Driver debug APK;
- iOS/iPadOS Driver Simulator ZIP;
- Docker/Compose を含む Linux Addressing Gateway x64 パッケージ;
- SHA256SUMS。

**Linux Control** はオペレーター用デスクトップアプリです。**Linux Addressing Gateway** は別のサーバー relay モジュールであり、異なるパッケージです。

Android と Apple のパッケージはまだストア用署名を行っていません。App Store/TestFlight/Google Play と production signing は後で別途対応します。

## 今後の作業

- Windows・macOS・Linux 向け Planner/editor デスクトップ UI とリリースビルド;

- ローカル browser の手動受け入れ;
- 同じ Driver Browser を実機の電話で同一 LAN 内テスト;
- HTTPS/public addressing path;
- HTTPS 経由での browser GPS 完全受け入れ;
- 異なるネットワーク/NAT/CGNAT でのフィールドテスト;


## インターフェース言語

Control、Driver Browser、Android Driver、iOS/iPadOS Driver は共通の **8言語セット: uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN** で実装・検証済みです。Control と Driver Browser はシステム言語と保存される手動選択をサポートし、CI が8言語すべての完全性を確認します。

## ドキュメント

公開状態の入口: [START_HERE.md](../../START_HERE.md)、[HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md)、[PROJECT_STATE.md](../../PROJECT_STATE.md)。

詳細な作業ドキュメントは非公開で管理されています。

## ライセンス

プロプライエタリソフトウェア © 2026 Roman Zavada。リポジトリが公開されていてもオープンソースの権利は付与されません。[LICENSE.md](../../LICENSE.md) と [COPYRIGHT.md](../../COPYRIGHT.md) を参照してください。
