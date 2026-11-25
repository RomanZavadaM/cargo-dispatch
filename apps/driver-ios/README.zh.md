# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · **🇨🇳 简体中文**

原生 Apple 司机客户端。一个 SwiftUI target 同时支持 iPhone 和 iPad，并与 Driver Browser/Android 使用**同一套 Control API**。

当前范围：Swift 6 + SwiftUI + URLSession；iOS/iPadOS 17+；Driver URL；trip snapshot；complete/reorder；永久 point ID + local revision；可选 road geometry；CoreLocation 前台一次性位置上报；Control→iPhone/iPad 文档下载/fileExporter 保存；通过 fileImporter + PhotosPicker 上传 iPhone/iPad→Control 文件/照片/视频；恢复最后的 Driver URL；本地化位置权限说明；uk/en/de/fr/es/ko/ja/zh-Hans；Swift 协议测试和 Simulator build/install/launch/screenshot CI。

本地 HTTP 仅允许 Simulator/LAN Debug；Release 保持 HTTPS-first。

后续：原生 MapKit 地图、相机拍摄、后台 GPS、连接真实 synthetic Control trip 的 smoke、离线队列/重试、Keychain 加固、push、生产签名/TestFlight。
