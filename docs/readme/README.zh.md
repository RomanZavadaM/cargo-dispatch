# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · **🇨🇳 简体中文**

> **状态：** 正在积极开发。当前稳定版本为 **v0.1.0**。完成全部本地化后，下一个主要方向是路线 Planner。

## 产品简介

**cargo-dispatch** 是面向货运车队的调度系统。目标包括选择货源、创建和重新规划行程、将行程交付执行，并让调度员掌握实际执行进度。

主要开发语言为 **Rust**。

## 当前已实现

- 基于 Tauri/Leptos 的独立桌面 **Control** 应用；
- 活跃行程的持久本地存储与重启恢复；
- Planner 与 Control 之间的版本化 trip package；
- 一个自适应 **Driver Browser**，适用于手机、平板、笔记本和桌面浏览器；
- 在 Control 和 Driver Browser 中调整未完成站点顺序；
- PDF 和普通文件 **Control → Driver Browser**；
- 文件、照片和视频 **Driver Browser → Control**，并可本地保存；
- 同一行程可同时连接两个浏览器客户端，并在无独立浏览器数据库的情况下重新连接；
- Control 和 Driver Browser 中的交互式 OpenStreetMap 地图；
- 用于测试和本地工作的 loopback 与显式 LAN 模式。

## 原生 Driver App

两个原生客户端都已经集成到 `main`：

- **Android** — Jetpack Compose、路线/完成/重排、文件/照片/视频、前台 GPS、会话恢复、原生 MapLibre + OpenStreetMap、当前道路 geometry、站点/最新位置和系统照片/视频拍摄；Emulator build/install/launch/screenshot PASS。
- **iPhone/iPad** — SwiftUI、路线/完成/重排、文件/照片/视频、前台 CoreLocation、会话恢复、原生 MapKit、当前 geometry、站点/最新位置和系统照片/视频拍摄；Simulator build/install/launch/screenshot PASS。

两者使用同一个 Control API，不创建独立的行程数据库。

r4/r5 之后，两个原生客户端还具备用于位置和媒体的持久重试队列。Android r5 增加了通过前台定位服务进行的显式 Start/Stop 后台行程跟踪、常驻 Stop 通知以及 15 秒 / 25 米策略。Apple r5 增加显式 Start/Stop CoreLocation 后台跟踪、`UIBackgroundModes=location`、25 米过滤，并且没有独立需求时不会请求 `Always` 权限。

## Driver Browser

Driver Browser 是**一个产品**，不是分别开发的手机版和桌面版。它有三种标准布局：

- 窄屏手机；
- 平板；
- 宽屏桌面/笔记本。

数据、路线操作、文件和 backend 完全共用，仅根据屏幕大小调整布局。

## 地图

Control 和 Driver Browser 已经包含真正的交互式基础地图，可显示行程站点和最后一次被接受的司机位置。

`main` 现在已经集成版本化的道路几何管线（schema 1.2）：Planner 可以携带派生 geometry，Control 会保存它，两端地图只在 geometry 与当前站点顺序一致时显示道路线。重新排序后，旧线会立即失效。

仍待完成的是**连接 Valhalla 的真实 HTTP transport 和 geometry 自动重算**。数据契约与显示层已经存在，但外部路由引擎尚未接入 product runtime。

## Stable v0.1.0

[打开 Stable v0.1.0](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0)。

Stable 从同一个精确源码重新构建并发布：
- Windows Control x64 ZIP；
- macOS Control universal ZIP — Intel + Apple Silicon；
- Linux Control x64 tar.gz；
- 用于测试安装的 Android Driver debug APK；
- iOS/iPadOS Driver Simulator ZIP；
- 带 Docker/Compose 的 Linux Addressing Gateway x64 包；
- SHA256SUMS。

**Linux Control** 是操作员桌面应用。**Linux Addressing Gateway** 是独立的服务器 relay 模块，两者是不同的软件包。

Android 和 Apple 软件包目前尚未进行商店签名。App Store/TestFlight/Google Play 和 production signing 将在之后单独处理。

## 后续工作

- 面向 Windows、macOS 和 Linux 的 Planner/editor 桌面 UI 与发布构建；

- 本地浏览器手动验收；
- 在同一 LAN 中用真实手机测试相同的 Driver Browser；
- HTTPS/公共寻址路径；
- 通过 HTTPS 完成浏览器 GPS 验收；
- 不同网络/NAT/CGNAT 的现场测试；
- 真实 Valhalla HTTP transport，以及重新排序后的 geometry 重算；


## 界面语言

当前已集成的 desktop/browser UI 实现并验证 **乌克兰语、英语和德语**。新的 Android 与 iOS/iPadOS 原生客户端从第一个界面起就包含 **uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN** 语言包，文档也使用同一组语言。

## 文档

公开状态入口：[START_HERE.md](../../START_HERE.md)、[HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md)、[PROJECT_STATE.md](../../PROJECT_STATE.md)。

详细工作文档以私有方式保存。

## 许可证

专有软件 © 2026 Roman Zavada。仓库公开可见并不授予开源权利。请参阅 [LICENSE.md](../../LICENSE.md) 和 [COPYRIGHT.md](../../COPYRIGHT.md)。
