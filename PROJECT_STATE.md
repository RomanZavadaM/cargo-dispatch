# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-24 23:05, Europe/Kyiv.

Це короткий публічний стан. Деталі, рішення й журнал — у private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Етап: product foundation інтегровано в `main`.
- Integration commit: `018c89c3ea85e56b5d533fb760ee26384abc9bac`.
- PR #1 merged з `product/foundation-r1`; product head `9b71f76424b3c9f4d3fb5f2634bc4df65517a406`.
- PR CI: Product foundation `45486110804` — **SUCCESS**; Control desktop UI `45486111221` — **SUCCESS**.
- Planner і Control — окремі desktop-програми.
- Мобільний клієнт першого етапу — Driver Browser, сторінку якого віддає Control.
- Уже перевірено автоматично: trip package, durable recovery, out-of-order replan, loopback/LAN session boundary, route/location, двосторонні PDF/file/photo/video, Tauri/Leptos Control dashboard та native Save.
- Наступне ручне приймання: реальний телефон у тій самій LAN; повний GPS acceptance — через HTTPS.
- Ще не готові: повноцінна карта в Control і Driver Browser, HTTPS/public gateway product path, field NAT/CGNAT test, real Valhalla route geometry, native Driver App, stable release.
- Опублікований `v0.1.0-preview.2` — лише тестовий UI preview, не stable product.
- UI-локалізації, фактично перевірені в prototype: Українська `uk`, English `en`, Deutsch `de`. Інші README-переклади не означають готову локалізацію програми.

Для продовження роботи — `START_HERE.md` → private `HANDOFF_CURRENT.md` / `PROJECT_STATE.md` → активні module logs → live GitHub.
