# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · **🇨🇳 简体中文**

原生 Apple 司机客户端。一个 SwiftUI target 同时支持 iPhone 和 iPad，并与 Driver Browser/Android 使用**同一套 Control API**。

当前范围：Swift 6 + SwiftUI + URLSession；iOS/iPadOS 17+；Driver URL；trip snapshot；complete/reorder；显示当前 road geometry、站点与最新位置的原生 MapKit；CoreLocation 前台一次性位置；文件/照片/视频传输与系统相机；用于最新位置/媒体的持久重试队列；使用 `UIBackgroundModes=location` 和 25 米 filter 的显式 Start/Stop 后台 CoreLocation tracking；安全会话恢复，token 存入不同步的 Keychain 项（`AfterFirstUnlockThisDeviceOnly`），UserDefaults 仅保存非敏感 base URL；成功重连后自动迁移旧 raw Driver URL；uk/en/de/fr/es/ko/ja/zh-Hans；Swift 协议测试以及连接真实 synthetic Control trip 的 Simulator smoke。

本地 HTTP 仅允许 Simulator/LAN Debug；Release 保持 HTTPS-first。

后续：真实设备 LAN/后台跟踪验收、HTTPS/public/NAT/CGNAT 现场路径、push/notifications、生产签名/TestFlight。
