# HANDOFF_CURRENT — cargo-dispatch

Оновлено: 2026-11-24 23:12, Europe/Kyiv.

Цей файл — короткий публічний якір. Повний поточний стан, правила й журнал ведуться приватно в `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

## Live public state

- `main`: product foundation інтегровано.
- Integration commit: `018c89c3ea85e56b5d533fb760ee26384abc9bac`.
- Public state-sync commit: `e133cfebd0a6523db5e9f1d40e345b18b8e72446`.
- PR #1 `product/foundation-r1 → main`: merged.
- Product head перед злиттям: `9b71f76424b3c9f4d3fb5f2634bc4df65517a406`.
- PR Product foundation run `45486110804`: **SUCCESS**.
- PR Control desktop UI run `45486111221`: **SUCCESS**.
- Опублікований test UI preview: `v0.1.0-preview.2`.
- Stable готового продукту ще немає.

## Що підтверджено

У `main` уже є:
- Planner → Control trip package;
- durable рейс і restart recovery;
- Control як сервер активного рейсу;
- tokenized loopback + explicit LAN;
- route/order, coordinates/address/navigation та driver location;
- PDF/file/photo/video в обидва боки;
- Tauri/Leptos Control dashboard з native folder/file picker і verified Save incoming media.

## Одна наступна дія

**Спочатку локальне приймання Driver Browser у звичайному browser window на тому самому комп'ютері через loopback Control.**

Важливо:
- тестується саме **мобільний Driver Browser UI**, без окремої desktop-версії, спрощень або підмін;
- той самий HTML/JS/WASM і той самий Control backend, що підуть на телефон;
- перевіряються route reorder, документи/PDF, фото/відео/файли, два одночасні browser clients і поведінка після refresh/reconnect;
- повноцінна карта має бути і у Control, і у Driver Browser; координати та navigation links її не замінюють.

Після локального browser acceptance — той самий UI на реальному телефоні в LAN. Повний GPS acceptance — через HTTPS path.

## Як продовжити

Спочатку відкрити private:
- `CARGO_DISPATCH_PROFILE.md`;
- `cargo-dispatch/HANDOFF_CURRENT.md`;
- `cargo-dispatch/PROJECT_STATE.md`;
- активні module STATE/WORKLOG.

Потім звірити live GitHub.
