# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · **🇨🇳 简体中文**

原生 Android 司机客户端。它**不会替代 Driver Browser**，也不会创建独立的服务器协议；它使用相同的 Control API 和行程规则。

当前范围：Kotlin + Jetpack Compose；Android 16/API 36（minSdk 26）；Driver URL；active-trip snapshot；complete/reorder；显示当前 road geometry、站点与最新位置的原生 MapLibre 地图；前台一次性位置；文件/照片/视频传输与系统相机；用于最新位置/媒体的持久重试队列；通过前台定位服务进行显式 Start/Stop 后台跟踪，策略为 15 秒 / 25 米；安全会话恢复，token 由 Android Keystore 使用 AES/GCM 加密，仅非敏感 base URL/Emulator mode 留在私有 preferences；成功重连后自动迁移旧 raw Driver URL；uk/en/de/fr/es/ko/ja/zh-CN 本地化；协议/单元测试以及连接真实 synthetic Control trip 的 Emulator smoke。

明文 HTTP 仅用于本地 debug；公开路径采用 HTTPS-first。

后续：真实设备 LAN/后台跟踪验收、HTTPS/public/NAT/CGNAT 现场路径、push/notifications、生产签名。
