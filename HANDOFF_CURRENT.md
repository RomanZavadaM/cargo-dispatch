# HANDOFF_CURRENT — cargo-dispatch

Оновлено: 2026-11-24 21:22, Europe/Kyiv.

Цей файл — короткий публічний якір. Повний поточний стан, правила й журнал ведуться приватно в `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

## Live public state

- `main`: `ee137b853c72a62caca1e1c5fec5a45ff155e671`.
- Активна product-робота: `product/foundation-r1`.
- Поточний product head: `dafb03ec7d1c4d37950c784d103ba24d38eeef6d`.
- Product CI run #87 / `45468910124`: SUCCESS на Linux, Windows і macOS.
- UI evidence/preview: `prototype/ui-r1`.
- Опублікований тестовий UI preview: `v0.1.0-preview.2`.
- Stable готового продукту ще немає.
- Open PR: немає.

## Що підтверджено

Product foundation уже має:
- версійований ZIP Planner → Control;
- durable папку рейсу та recovery після restart;
- out-of-order completion і повний replan round-trip;
- двосторонні документи в межах рейсу;
- перевірене завершення/архівування;
- локальний browser access, який віддає сам Control на `127.0.0.1`, з tokenized trip session.

LAN test mode, public gateway/WebRTC, real Valhalla adapter і фінальна UI composition ще не готові.

## Як продовжити

Спочатку відкрити private:
- `CARGO_DISPATCH_PROFILE.md`;
- `cargo-dispatch/HANDOFF_CURRENT.md`;
- `cargo-dispatch/PROJECT_STATE.md`;
- останній запис `cargo-dispatch/WORKLOG.md`.

Потім звірити live GitHub. Якщо короткий публічний якір відстав — live GitHub + private HANDOFF мають пріоритет.
