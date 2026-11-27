# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · **🇨🇳 简体中文**

原生 Apple 司机客户端。一个 SwiftUI target 同时支持 iPhone 和 iPad，并与 Driver Browser/Android 使用**同一套 Control API**。

当前范围：Swift 6 + SwiftUI + URLSession；iOS/iPadOS 17+；Driver URL；trip snapshot；complete/reorder；显示当前 road geometry、站点与最新位置的原生 MapKit；CoreLocation 前台一次性位置；文件/照片/视频传输与系统相机；用于位置和媒体的持久重试队列；使用 `UIBackgroundModes=location` 和 25 米 filter 的显式 Start/Stop 后台 CoreLocation tracking；安全会话恢复，token 存入不同步的 Keychain 项（`AfterFirstUnlockThisDeviceOnly`），UserDefaults 仅保存非敏感 base URL；成功重连后自动迁移旧 raw Driver URL；uk/en/de/fr/es/ko/ja/zh-Hans；Swift 协议测试以及连接真实 synthetic Control trip 的 Simulator smoke。

本地 HTTP 仅允许 Simulator/LAN Debug；Release 保持 HTTPS-first。

**司机日常使用**

- 每个未完成地点旁有“在导航中打开”按钮；
- “标记完成”需要确认；
- 新文件可关联到某个地点或整个行程；
- 界面打开时每 15 秒自动刷新，并显示是否已连接；
- 无网络时保存行驶轨迹（最多最近 500 个位置），恢复后按顺序发送；
- 被调度程序拒收的文件会保留，可重新发送或移除；
- 错误提示使用手机语言，不含内部术语。

后续：真实设备 LAN/后台跟踪验收、HTTPS/public/NAT/CGNAT 现场路径、push/notifications、生产签名/TestFlight。
