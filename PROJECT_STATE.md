# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-24 23:18, Europe/Kyiv.

Це короткий публічний стан. Деталі, рішення й журнал — у private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Product foundation інтегровано в `main`.
- Integration commit: `018c89c3ea85e56b5d533fb760ee26384abc9bac`.
- PR CI: Product foundation `45486110804` — **SUCCESS**; Control desktop UI `45486111221` — **SUCCESS**.
- Planner і Control — окремі desktop-програми.
- Driver Browser — один адаптивний browser client для телефона, планшета, ноутбука й desktop browser; layout змінюється за шириною, функції та backend спільні.
- Уже перевірено автоматично: trip package, durable recovery, out-of-order replan, loopback/LAN session boundary, route/location, двосторонні PDF/file/photo/video, Tauri/Leptos Control dashboard та native Save.
- **Поточне ручне приймання:** локально через loopback перевірити wide desktop/laptop, tablet і narrow phone layouts без втрати або спотворення mobile UI.
- Після local responsive PASS — реальний телефон у LAN; повний GPS acceptance — через HTTPS.
- Ще не готові: повноцінна карта в Control і Driver Browser, HTTPS/public gateway product path, field NAT/CGNAT test, real Valhalla route geometry, native Driver App, stable release.
- `v0.1.0-preview.2` — test UI preview, не stable product.

Для продовження роботи — `START_HERE.md` → private HANDOFF/PROJECT_STATE → active module logs → live GitHub.
