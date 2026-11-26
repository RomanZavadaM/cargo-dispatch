# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · **🇨🇳 简体中文**

> **状态：** 正在积极开发。当前稳定版本为 **v0.1.0**。当前测试 prerelease 为 **v0.1.0-preview.7**，已经包含面向 Windows、macOS 和 Linux 的独立 Planner。完整的 8 语言本地化已集成。

## 产品简介

**cargo-dispatch** 是面向货运车队的调度系统。目标包括选择货源、创建和重新规划行程、将行程交付执行，并让调度员掌握实际执行进度。

主要开发语言为 **Rust**。

## 当前已实现

- 基于 Tauri/Leptos 的独立桌面 **Control** 应用；
- 基于 Tauri/Leptos 的独立桌面 **Planner**：手动路线草稿、站点添加/删除/重排、导航坐标，以及由 Planner core 管理的 revision；草稿保存在 app-data 中的 versioned append-only journal，并可在重启后恢复；
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

当前 `main` 使用带有不可变 planning basis 的 schema 1.4，并继续读取旧 schema。Planner routing profile 只是某一次规划计算的输入，不会把行程永久绑定到车辆。Import 之后，Control 会单独维护活动会话真实且可变的 routing context。

Control 保存派生 geometry，并在站点顺序变化后通过已配置的 runtime Valhalla 重新计算道路线。延迟返回的 routing 响应不能覆盖更新的行程状态。即使 Valhalla 未配置或暂时不可用，新的站点顺序仍会保存，只是派生 geometry 可以暂时不存在。

Planner desktop 现在拥有自己的 runtime Valhalla preview：操作员设置 Valhalla 地址、地图数据集 ID 和 timeout，通过 `/status` 检查可用性，`/route` 只使用明确提供的 planning profile 计算。未知的数值约束不会被猜测。成功结果会在 OSM 上绘制真实道路 polyline；没有结果时不会绘制虚假的直线。Preview 由 route/revision/profile 派生：草稿或 planning profile 改变后旧线路会被清除，延迟响应也会被拒绝。Routing 失败不会修改 Planner 草稿。

## Test v0.1.0-preview.7

[打开 Test v0.1.0-preview.7](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.7)。

这是 **prerelease**，不是 Stable。这是第一个从同一精确源码同时发布独立桌面 Planner 与其他产品组件的集成 preview。

Release 包含：
- Windows Control x64 ZIP；
- macOS Control universal ZIP — Intel + Apple Silicon；
- Linux Control x64 tar.gz；
- Windows Planner x64 ZIP；
- macOS Planner universal ZIP — Intel + Apple Silicon；
- Linux Planner x64 tar.gz；
- Android Driver debug APK；
- iOS/iPadOS Driver Simulator ZIP；
- Linux Addressing Gateway x64 package；
- SHA256SUMS。

第一个 Planner slice 支持手动路线草稿和站点编辑。在 preview.7 中，Planner 草稿尚不能在重启后保留；这是下一阶段。

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

- 通过现有 versioned trip package 从 Planner UI 导出 package；

- 本地浏览器手动验收；
- 在同一 LAN 中用真实手机测试相同的 Driver Browser；
- HTTPS/公共寻址路径；
- 通过 HTTPS 完成浏览器 GPS 验收；
- 不同网络/NAT/CGNAT 的现场测试；


## 界面语言

Control、Planner、Driver Browser、Android Driver 和 iOS/iPadOS Driver 已使用同一套 **8 种语言：uk、en、de、fr、es、ko、ja、zh-Hans/zh-CN** 完成实现与验证。Control 和 Driver Browser 支持系统语言与持久化手动选择，CI 会检查全部 8 个本地化的完整性。

## 文档

公开状态入口：[START_HERE.md](../../START_HERE.md)、[HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md)、[PROJECT_STATE.md](../../PROJECT_STATE.md)。

详细工作文档以私有方式保存。

## 许可证

专有软件 © 2026 Roman Zavada。仓库公开可见并不授予开源权利。请参阅 [LICENSE.md](../../LICENSE.md) 和 [COPYRIGHT.md](../../COPYRIGHT.md)。
