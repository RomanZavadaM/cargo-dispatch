# HANDOFF_CURRENT — cargo-dispatch

Оновлено: 2026-11-24 23:05, Europe/Kyiv.

Цей файл — короткий публічний якір. Повний поточний стан, правила й журнал ведуться приватно в `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

## Live public state

- `main`: product foundation інтегровано; integration commit `018c89c3ea85e56b5d533fb760ee26384abc9bac`.
- PR #1 `product/foundation-r1 → main`: merged.
- Product head перед злиттям: `9b71f76424b3c9f4d3fb5f2634bc4df65517a406`.
- PR Product foundation run `45486110804`: **SUCCESS** — Linux quality/dependencies + Windows/macOS portability/tests.
- PR Control desktop UI run `45486111221`: **SUCCESS** — Leptos frontend + Tauri Ubuntu/Windows/macOS check/clippy/tests.
- UI evidence/preview: `prototype/ui-r1`.
- Опублікований тестовий UI preview: `v0.1.0-preview.2`.
- Stable готового продукту ще немає.
- Open PR: немає.

## Що підтверджено

У `main` уже є:
- versioned Planner → Control trip package;
- durable рейс, restart recovery, out-of-order completion/replan;
- Control як сервер активного рейсу для Driver Browser;
- loopback + explicit private-LAN test mode з tokenized session;
- route/order, coordinates/address/navigation та driver location;
- PDF/file/photo/video в обидва боки;
- реальний Tauri/Leptos Control dashboard з native folder/file picker і verified Save incoming media.

## Одна наступна дія

Manual real-phone LAN acceptance: відкрити driver URL на реальному телефоні в тій самій мережі й перевірити route reorder, Control→phone PDF/file, phone→Control photo/video/file, native Save та два browser clients одного trip.

GPS через plain LAN HTTP не вважати повністю прийнятим; повний GPS acceptance — через HTTPS path.

Окремо зафіксована UI-вимога власника: повноцінна карта має бути і у вікні Control, і у мобільному Driver Browser; поточні координати/навігаційне посилання цю вимогу ще не закривають.

## Як продовжити

Спочатку відкрити private:
- `CARGO_DISPATCH_PROFILE.md`;
- `cargo-dispatch/HANDOFF_CURRENT.md`;
- `cargo-dispatch/PROJECT_STATE.md`;
- активні module STATE/WORKLOG.

Потім звірити live GitHub. Якщо короткий публічний якір відстав — live GitHub + private HANDOFF мають пріоритет.
