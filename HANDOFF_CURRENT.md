# HANDOFF_CURRENT — cargo-dispatch

Оновлено: 2026-11-26, Europe/Kyiv.

Це короткий public anchor. Повний стан і журнали — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

## Release checkpoints
- Поточна Stable: `v0.1.0`.
- Поточний Test/prerelease: `v0.1.0-preview.7`.
- Exact source `preview.7`: `db3d6f7450bd886f80f8a48221f6254394fad08e`.
- `preview.7` містить 10 release assets:
  - Control Windows x64 / macOS universal / Linux x64;
  - Planner Windows x64 / macOS universal / Linux x64;
  - Android Driver APK;
  - iOS/iPadOS Driver Simulator ZIP;
  - Linux Addressing Gateway;
  - `SHA256SUMS.txt`.
- Mobile store signing/TestFlight/Google Play поки відкладені.

## Інтегровано
- Control + Driver Browser з durable trip state, maps, files/media, location, route reorder і reconnect.
- Android/iOS Driver із native maps, camera/files, durable retry, background tracking і secure session storage.
- Addressing Gateway — RAM-only relay; Control лишається джерелом активного рейсу.
- Control desktop: Windows + macOS + Linux.
- Planner desktop: Windows + macOS + Linux.
- Planner manual route draft:
  - add/remove/reorder точок;
  - navigation coordinates;
  - revision через Planner core;
  - 8 мов;
  - versioned append-only journal в app-data;
  - відновлення чернетки, порядку точок, revision та ID після перезапуску;
  - обірваний незавершений останній journal record після аварії ігнорується.
- Full 8-language localization контролюється CI.
- Planner / routing foundation:
  - Valhalla adapter;
  - trip schema 1.3 з actual VehicleRoutingProfile; legacy 1.0–1.2 readable;
  - real Valhalla HTTP transport;
  - runtime URL / map dataset id / bounded connect + request timeout у Desktop Control;
  - `/status` availability + live engine version;
  - live geometry recalculation після operator/driver reorder;
  - stale/late Valhalla response не може перезаписати новіший active-trip state.

## Next
1. Map/routing preview у Planner desktop UI.
2. Далі package export із Planner UI через наявний versioned trip package.
3. Geometry лишається derived; Control лишається authoritative після імпорту рейсу.
4. Compiled test/stable builds публікувати через GitHub Releases, не залишати тільки в Actions artifacts.

Продовження: `START_HERE.md` → private HANDOFF/PROJECT_STATE → Planner STATE/WORKLOG → live GitHub.
