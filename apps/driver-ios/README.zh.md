# Cargo Dispatch Driver — iOS/iPadOS 骨架

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · **🇨🇳 简体中文**

首个原生 Apple 开发阶段。一个 SwiftUI app target 同时支持 iPhone 和 iPad，并与 Driver Browser 和 Android Driver App 使用**同一套 Control API**。

当前范围：Swift 6 + SwiftUI + URLSession、iOS/iPadOS 17+、Driver URL、trip snapshot、complete/reorder、永久 point ID + local revision、可选路线几何模型、纯 Swift core test，以及 uk/en/de/fr/es/ko/ja/zh-Hans 本地化。

本地 HTTP 仅允许 Simulator/LAN Debug；Release 保持 HTTPS-first。

尚未实现：原生地图、相机/照片/视频、文档打开/分享、后台 GPS、离线队列、安全持久会话、push、production signing/TestFlight。
