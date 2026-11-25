# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · **🇯🇵 日本語** · [🇨🇳 简体中文](README.zh.md)

> **ステータス:** 開発中です。現在のテスト用プレリリースは **v0.1.0-preview.3** です。安定版の完成製品はまだありません。

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

ただし、これはまだ **完全な道路ルート形状ではありません**。Valhalla による実際のルート形状の統合は今後の作業です。

## テストビルド

現在のプレリリース: [v0.1.0-preview.3](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.3).

Windows ZIP には Control、合成されたアクティブ運行、ローカル loopback 検証用のサンプル PDF が含まれます。

これは **test/prerelease build** であり、stable ではありません。

## 今後の作業

- ローカル browser の手動受け入れ;
- 同じ Driver Browser を実機の電話で同一 LAN 内テスト;
- HTTPS/public addressing path;
- HTTPS 経由での browser GPS 完全受け入れ;
- 異なるネットワーク/NAT/CGNAT でのフィールドテスト;
- 実際の Valhalla 道路ルート形状;
- 将来の native Driver App;
- stable release.

## インターフェース言語

現在の UI で実装・検証済みなのは **ウクライナ語、英語、ドイツ語** です。その他の README 翻訳は文書用または将来の対象言語であり、現時点でプログラム UI が対応していることを意味しません。

## ドキュメント

公開状態の入口: [START_HERE.md](../../START_HERE.md)、[HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md)、[PROJECT_STATE.md](../../PROJECT_STATE.md)。

詳細な作業ドキュメントは非公開で管理されています。

## ライセンス

プロプライエタリソフトウェア © 2026 Roman Zavada。リポジトリが公開されていてもオープンソースの権利は付与されません。[LICENSE.md](../../LICENSE.md) と [COPYRIGHT.md](../../COPYRIGHT.md) を参照してください。
