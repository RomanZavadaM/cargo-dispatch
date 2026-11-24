# HANDOFF_CURRENT — cargo-dispatch

Оновлено: 2026-11-24 23:18, Europe/Kyiv.

Цей файл — короткий публічний якір. Повний поточний стан, правила й журнал ведуться приватно в `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

## Live public state

- `main`: product foundation інтегровано.
- Integration commit: `018c89c3ea85e56b5d533fb760ee26384abc9bac`.
- PR #1 merged.
- Product head перед злиттям: `9b71f76424b3c9f4d3fb5f2634bc4df65517a406`.
- PR Product foundation `45486110804`: **SUCCESS**.
- PR Control desktop UI `45486111221`: **SUCCESS**.
- Test UI preview: `v0.1.0-preview.2`.
- Stable готового продукту ще немає.

## Driver Browser

Driver Browser — один браузерний продукт з адаптивним інтерфейсом:
- phone layout для телефона;
- tablet layout для планшета;
- wide/desktop layout для ноутбука або великого екрана.

Це не три різні клієнти: дані, функції та backend однакові; змінюється компонування під розмір екрана. Desktop/tablet режим не скасований.

## Одна наступна дія

**Local responsive browser acceptance через loopback Control на тому самому комп'ютері.**

Перевірити:
- wide desktop/laptop layout;
- tablet-size layout;
- narrow phone layout без спотворення мобільного інтерфейсу;
- однаковий функціонал у всіх режимах: route reorder, PDF/file, photo/video/file, native Save у Control, два browser clients, refresh/reconnect;
- повноцінну карту у Control і Driver Browser не підміняти списком координат.

Після local PASS — той самий Driver Browser на реальному телефоні в LAN. Повний GPS acceptance — через HTTPS path.

## Як продовжити

`START_HERE.md` → private `HANDOFF_CURRENT.md` / `PROJECT_STATE.md` → active module STATE/WORKLOG → live GitHub.
