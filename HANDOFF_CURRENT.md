# HANDOFF_CURRENT — cargo-dispatch

Оновлено: 2026-11-25, Europe/Kyiv.

Цей файл — короткий публічний якір. Повний поточний стан, правила й журнал ведуться приватно в `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

## Live public state

- PR #2 `Prepare responsive local Driver Browser acceptance` — merged.
- Integration merge commit: `f41d32044f8ac0eb3290c9638a58f4c7f8046d77`.
- Exact product head перед merge: `2d10f0db81346311525861d948104caf8a8fc1ac`.
- PR Product foundation run `45571225967`: **SUCCESS**.
- PR Control desktop UI run `45571226378`: **SUCCESS**.
- Поточний test/prerelease: `v0.1.0-preview.3`.
- Stable готового продукту ще немає.

## Control + Driver Browser

Control уже працює з durable active-trip, loopback/LAN session, route actions, location і двосторонніми файлами/media.

Driver Browser — один адаптивний продукт:
- phone layout;
- tablet layout;
- wide desktop/laptop layout.

У Control і Driver Browser є інтерактивна OpenStreetMap-карта з route points та останньою прийнятою позицією водія.

## Поточна межа приймання

Manual local browser acceptance ще потрібен для:
- wide/tablet/narrow;
- route reorder в обидва боки;
- PDF/file Control → Driver Browser;
- file/photo/video Driver Browser → Control + native Save;
- два browser clients;
- refresh/reconnect;
- карти в Control і Driver Browser.

Після local PASS — той самий Driver Browser на реальному телефоні в LAN. Повний GPS acceptance — через HTTPS.

## Ще попереду

- real Valhalla road geometry;
- HTTPS/public addressing path;
- різні мережі/NAT/CGNAT;
- native Driver App;
- stable release.

## Як продовжити

`START_HERE.md` → private `HANDOFF_CURRENT.md` / `PROJECT_STATE.md` → active module STATE/WORKLOG → live GitHub.
