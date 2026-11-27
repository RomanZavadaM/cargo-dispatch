# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · **🇨🇳 简体中文**

货运车队调度系统：规划行程、交付执行、与司机保持联系，并记录实际情况。主要开发语言为 **Rust**。

**稳定版本：[v0.3.0](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.3.0)。** 开发仍在继续；测试版本见 [Releases](https://github.com/RomanZavadaM/cargo-dispatch/releases)。

## 组成

| 程序 | 使用者 | 平台 |
|---|---|---|
| **Control** | 调度员：管理进行中的行程 | Windows、macOS、Linux |
| **Planner** | 计划员：编制路线和行程包 | Windows、macOS、Linux |
| **司机页面** | 司机，任意浏览器，无需安装 | 手机、平板、电脑 |
| **Android 版 Driver** | 司机 | Android 8.0 及以上 |
| **iPhone 和 iPad 版 Driver** | 司机 | iOS / iPadOS 17 及以上 |
| **Addressing Gateway** | 通过互联网连接 | Linux 服务器 |

进行中行程的实际状态只以 Control 为准。它直接向司机提供页面、路线和文件，并接收完成标记、位置和附件。没有中央行程数据库。

## 快速开始

1. 从发布页面下载压缩包，解压后启动 **Control**。Control 安装包内含虚构的测试数据（`local-browser-test-data`）。
2. **“添加行程”** →“打开现有行程”（行程文件夹或 Planner 行程包）或“手动创建新行程”。
3. **“连接司机”** →“本机”“同一 Wi-Fi 网络”或“任何地点，通过互联网”。
4. 把 **“下一步”** 区域中的链接交给司机：“打开”或“复制”。
5. 司机在浏览器中打开链接，或将其粘贴到 Driver 应用。

## 功能

**Control**
- 同时管理多个行程，每个行程可单独开窗口；
- 行程来自 Planner 行程包或手动创建；状态保存在本地，重启后恢复；
- 未完成地点的顺序、地图、司机最新位置、计划与实际对比；
- 向司机发送文件；接收司机的文件、照片和视频；
- 自动识别本机在局域网中的地址；切换连接方式无需手动关闭；
- 错误提示使用界面语言；位置时间、文件大小和地点名称以易懂方式显示。

**Planner**
- 订单文件（JSON，人与 AI 通用）或手动录入；按载量给出最多三个路线方案，选定后生成行程；输入内容重启后保留。
- 路线草稿：按坐标添加、删除、重新排序地点；草稿在重启后保留；
- 使用明确填写的车辆参数，通过您自己的 Valhalla 服务器预览路线和道路线（未知数值不会编造）；
- 导出包含货物和计划依据的行程包（ZIP，架构 1.4）；从不覆盖已有文件。

**司机：浏览器、Android、iPhone 和 iPad**
- 路线和地图、标记地点完成、调整未完成地点顺序；
- 接收文件资料；发送文件、照片和视频；
- 上报位置；应用中还支持后台上报，需明确开启和关闭，并带有重发队列；
- 标记地点完成前需要确认；应用还提供导航按钮、将文件关联到地点、界面自动刷新、无网络时保存行驶轨迹（最多 500 个位置）以及显示被拒收的文件。

## 连接与安全

- **本机**和**局域网**方式不加密（HTTP）：只在可信网络中使用。网络访问不会自行开启，也不会打开路由器端口。
- 带有密钥的链接即可访问行程：只发给该行程的司机。
- 手机浏览器通常只在 HTTPS（即通过 Gateway）下允许定位。
- 要让手机在网络中访问 Control，请在电脑防火墙中放行该程序。
- **通过互联网：** 在 HTTPS 反向代理后部署您自己的 Gateway。由 Control 主动连接，电脑无需开放入站端口。Gateway 不保存行程、文件、媒体或位置。默认单次请求中继上限为 64 MiB（最高可设为 512 MiB）。

## 地图与路线

Control、Planner、浏览器和 Android 使用 [OpenStreetMap](https://www.openstreetmap.org/copyright)（© OpenStreetMap contributors）地图；iPhone 和 iPad 使用 Apple MapKit。地图提供方可以看到设备 IP 地址和所显示的区域。道路线需要您自己的 Valhalla 服务器；没有它行程照常工作，只是不显示道路线。

## 语言

所有程序支持 українська、English、Français、Deutsch、Español、한국어、日本語和简体中文。CI 检查翻译是否完整；错误信息中的系统技术细节仍为英文。

## 尚未验证的内容与限制

- Android 和 iPhone/iPad 应用尚未在实体设备上测试，仅经过模拟器和自动测试。
- 移动网络以及真实网络中的 Gateway 仅经过自动测试。
- 安装包未签名；Android 为调试版 APK，iOS 仅有 Apple 模拟器版本。
- Control 重启后行程和链接自动恢复，连接方式点一次即可开启。使用 Gateway 时，需先重新输入其地址和密钥。
- 调度员暂时无法编辑地点或标记完成。

## 构建

构建、测试和发布在 GitHub Actions 中进行；见 `.github/workflows`。

## 文档

项目状态（乌克兰语）：[START_HERE.md](../../START_HERE.md)、[HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md)、[PROJECT_STATE.md](../../PROJECT_STATE.md)。详细工作文档不公开。

## 许可证

专有软件 © 2026 Roman Zavada (Роман Завада)。仓库公开并不授予开源权利。见 [LICENSE.md](../../LICENSE.md)、[COPYRIGHT.md](../../COPYRIGHT.md) 和 [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md)。
