# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-24 21:22, Europe/Kyiv.

Це короткий публічний стан. Деталі, рішення й журнал — у private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Етап: активна розробка product foundation.
- Мова: Rust.
- Planner і Control — окремі desktop-програми.
- Мобільний клієнт першого етапу — browser, сторінку якого віддає Control.
- Активна гілка: `product/foundation-r1`.
- Product head: `dafb03ec7d1c4d37950c784d103ba24d38eeef6d`.
- Product CI #87 / `45468910124`: SUCCESS на Linux, Windows і macOS.
- Уже перевірено: ZIP trip package, durable recovery, out-of-order replan round-trip, документи в обидва боки, завершення/архів, loopback browser access на `127.0.0.1`.
- Ще не готові: explicit LAN test mode, реальний телефон, public gateway/WebRTC product adapter, real Valhalla adapter, фінальна desktop UI composition.
- Опублікований `v0.1.0-preview.2` — лише тестовий UI preview, не stable product.
- UI-локалізації, фактично перевірені в prototype: Українська `uk`, English `en`, Deutsch `de`. Інші README-переклади не означають готову локалізацію програми.

Для продовження роботи — `START_HERE.md` → private `HANDOFF_CURRENT.md` / `PROJECT_STATE.md` → звірка live GitHub.
