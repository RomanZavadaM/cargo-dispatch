# Cargo Dispatch Driver — Android 骨架

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · **🇨🇳 简体中文**

首个原生 Android 开发阶段。它**不会替代 Driver Browser**，也不会创建第二套服务器协议；它使用相同的 Control API。

当前范围：Kotlin + Jetpack Compose、Android 16/API 36、Driver URL 连接、active trip snapshot、complete/reorder、路线几何模型、重连/错误状态、协议/单元测试，以及 uk/en/de/fr/es/ko/ja/zh-CN 本地化。

在 Android Emulator 中，主机的 `127.0.0.1`/localhost 映射到 `10.0.2.2`。明文 HTTP 仅允许本地 debug；公开路径必须使用 HTTPS。

尚未实现：相机/照片/视频、文档打开/分享、后台 GPS、离线队列、安全持久会话、push、生产签名。
