# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · **🇨🇳 简体中文**

原生 Android 司机客户端。它**不会替代 Driver Browser**，也不会创建独立的服务器协议；它使用相同的 Control API 和行程规则。

当前范围：Kotlin + Jetpack Compose；Android 16/API 36（minSdk 26）；Driver URL；Emulator 通过 `10.0.2.2` 访问主机 loopback；active-trip snapshot；complete/reorder；可选 road geometry；前台一次性位置上报；Control→Android 文档下载与原生保存；通过系统选择器上传 Android→Control 文件/照片/视频；恢复最后的 Driver URL/Emulator 模式；uk/en/de/fr/es/ko/ja/zh-CN 本地化；协议/单元测试以及 Emulator 安装/启动/截图 CI。

明文 HTTP 仅用于本地 debug；公开路径采用 HTTPS-first。

后续：原生地图、相机拍摄、后台 GPS、连接真实 synthetic Control trip 的 smoke、离线队列/重试、Keystore 加固、push、生产签名。
