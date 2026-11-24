# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-24 23:12, Europe/Kyiv.

Це короткий публічний стан. Деталі, рішення й журнал — у private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Product foundation інтегровано в `main`.
- Integration commit: `018c89c3ea85e56b5d533fb760ee26384abc9bac`.
- PR #1 merged; product head `9b71f76424b3c9f4d3fb5f2634bc4df65517a406`.
- PR CI: Product foundation `45486110804` — **SUCCESS**; Control desktop UI `45486111221` — **SUCCESS**.
- Planner і Control — окремі desktop-програми.
- Мобільний клієнт першого етапу — Driver Browser, сторінку якого віддає Control.
- Уже перевірено автоматично: trip package, durable recovery, out-of-order replan, loopback/LAN session boundary, route/location, двосторонні PDF/file/photo/video, Tauri/Leptos Control dashboard та native Save.
- **Поточне ручне приймання:** локальний browser на тому самому комп'ютері через loopback, але з незміненим mobile Driver Browser UI і тим самим backend.
- Після локального acceptance — реальний телефон у LAN; повний GPS acceptance — через HTTPS.
- Ще не готові: повноцінна карта в Control і Driver Browser, HTTPS/public gateway product path, field NAT/CGNAT test, real Valhalla route geometry, native Driver App, stable release.
- Опублікований `v0.1.0-preview.2` — лише test UI preview, не stable product.

Для продовження роботи — `START_HERE.md` → private HANDOFF/PROJECT_STATE → активні module logs → live GitHub.
