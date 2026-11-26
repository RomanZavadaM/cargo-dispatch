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
  - Valhalla adapter і real HTTP transport;
  - trip schema 1.4 з immutable planning basis; legacy schema залишаються читабельними;
  - Planner routing profile — лише параметри конкретного планового розрахунку, а не прив'язка рейсу до автомобіля;
  - Control зберігає окремий mutable actual routing context для активної сесії;
  - runtime URL / map dataset id / bounded connect + request timeout у Desktop Control;
  - Control `/status` availability + live geometry recalculation після operator/driver reorder;
  - stale/late Valhalla response не може перезаписати новіший active-trip state;
  - Planner desktop має власний runtime Valhalla preview: `/status`, real `/route`, explicit planning profile, реальна дорожня polyline поверх OSM;
  - невдала або застаріла відповідь Planner routing не змінює manual draft чи його revision.

## Next
1. Package export із Planner UI через наявний versioned trip package.
2. Передавати planning basis та потрібну derived geometry як snapshot пакета, не як постійну прив'язку рейсу до авто.
3. Geometry лишається derived; Control лишається authoritative після імпорту рейсу і веде окремий actual routing context.
4. Compiled test/stable builds публікувати через GitHub Releases, не залишати тільки в Actions artifacts.

Продовження: `START_HERE.md` → private HANDOFF/PROJECT_STATE → Planner STATE/WORKLOG → live GitHub.
