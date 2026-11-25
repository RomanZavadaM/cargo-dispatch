# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-25, Europe/Kyiv.

Це короткий публічний стан. Деталі, рішення й журнал — у private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Product foundation інтегровано в `main`.
- Responsive Control + Driver Browser slice з PR #2 також інтегровано в `main`; merge commit: `f41d32044f8ac0eb3290c9638a58f4c7f8046d77`.
- Exact-head PR CI перед merge: Product foundation `45571225967` — **SUCCESS**; Control desktop UI `45571226378` — **SUCCESS**.
- Planner і Control — окремі desktop-компоненти; після передачі активного trip фактом володіє Control.
- Driver Browser — один адаптивний browser client для телефона, планшета, ноутбука й desktop browser; backend та дані спільні.
- У Control і Driver Browser є інтерактивна OpenStreetMap-карта з точками рейсу та останньою прийнятою позицією водія.
- Уже перевірено автоматично: trip package, durable recovery, route reorder, loopback/LAN session boundary, route/location, двосторонні PDF/file/photo/video, два browser clients, reconnect, Tauri/Leptos Control dashboard та native Save.
- Поточний test/prerelease: **v0.1.0-preview.3**; stable product release ще немає.
- Ручне local browser acceptance ще не завершене; після нього — реальний телефон у LAN.
- Повний browser GPS acceptance — через HTTPS.
- Ще не готові: production HTTPS/public gateway path, field NAT/CGNAT acceptance, real Valhalla road geometry, native Driver App, stable release.

Для продовження роботи: `START_HERE.md` → private HANDOFF/PROJECT_STATE → active module logs → live GitHub.
