# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · **🇯🇵 日本語** · [🇨🇳 简体中文](README.zh.md)

> **ステータス:** 開発中です。現在のテスト用プレリリースは **v0.1.0-preview.4** です。安定版の完成製品はまだありません。

## 製品について

**cargo-dispatch** は、小型トラック車両群（車両総重量 7.5 t 以下）向けの配車システムです。貨物の選定、運行の作成・再計画、実行への引き渡し、そして配車担当者による実際の進行管理を目的としています。

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

## Driver Browser

Driver Browser は電話版と desktop 版に分かれた製品ではなく、**1 つの製品**です。標準レイアウトは次の 3 つです。

- narrow phone;
- tablet;
- wide desktop/laptop.

データ、経路操作、ファイル、backend は共通で、画面サイズに応じて配置だけが変わります。

## 地図

Control と Driver Browser には、運行地点と最後に受理されたドライバー位置を表示する実際のインタラクティブなベースマップがあります。

`main` には現在、バージョン付き道路形状パイプライン（schema 1.2）が統合されています。Planner は派生 geometry を渡せ、Control が保存し、両方の地図は現在の地点順序と一致する間だけ道路線を表示します。Reorder 後は古い線を直ちに無効化します。

残っているのは **Valhalla への実 HTTP transport と geometry の自動再計算**です。契約と表示は実装済みですが、外部ルーティングエンジンはまだ product runtime に接続されていません。

## テストビルド

現在のプレリリース: [v0.1.0-preview.4](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.4).

Windows ZIP には Control、合成されたアクティブ運行、ローカル loopback 検証用のサンプル PDF が含まれます。

これは **test/prerelease build** であり、stable ではありません。Preview.4 には Valhalla adapter と schema 1.2 road-geometry pipeline が含まれています。その後 Android と iOS/iPadOS の native Driver r2 が統合され、`main` は再び preview.4 より先に進んでいます。

## 今後の作業

- ローカル browser の手動受け入れ;
- 同じ Driver Browser を実機の電話で同一 LAN 内テスト;
- HTTPS/public addressing path;
- HTTPS 経由での browser GPS 完全受け入れ;
- 異なるネットワーク/NAT/CGNAT でのフィールドテスト;
- 実 Valhalla HTTP transport と reorder 後の geometry 再計算;
- native map：Android は MapLibre、iOS/iPadOS は MapKit;\n- カメラ撮影、background GPS、接続済み virtual-device smoke、offline queue、安全な session storage;
- stable release.

## インターフェース言語

現在統合済みの desktop/browser UI は **ウクライナ語、英語、ドイツ語** を実装・検証しています。新しい Android と iOS/iPadOS の native client は最初の画面から **uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN** の言語パックを持ち、ドキュメントも同じ言語セットで管理します。

## ドキュメント

公開状態の入口: [START_HERE.md](../../START_HERE.md)、[HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md)、[PROJECT_STATE.md](../../PROJECT_STATE.md)。

詳細な作業ドキュメントは非公開で管理されています。

## ライセンス

プロプライエタリソフトウェア © 2026 Roman Zavada。リポジトリが公開されていてもオープンソースの権利は付与されません。[LICENSE.md](../../LICENSE.md) と [COPYRIGHT.md](../../COPYRIGHT.md) を参照してください。
