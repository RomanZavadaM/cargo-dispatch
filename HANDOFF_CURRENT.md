# HANDOFF_CURRENT — cargo-dispatch

Оновлено: 2026-11-26, Europe/Kyiv.

Це короткий public anchor. Повний стан і журнали — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

## Stable checkpoint
- Поточна стабільна версія: `v0.1.0`.
- Stable містить Control Windows x64, macOS universal, Linux x64, Android Driver APK, iOS/iPadOS Driver Simulator ZIP, Linux Addressing Gateway і `SHA256SUMS.txt`.
- Mobile store signing/TestFlight/Google Play поки відкладені.

## Інтегровано
- Control + Driver Browser з durable trip state, maps, files/media, location, route reorder і reconnect.
- Android/iOS Driver із native maps, camera/files, durable retry, background tracking і secure session storage.
- Addressing Gateway — RAM-only relay; Control лишається джерелом активного рейсу.
- Control desktop: Windows + macOS + Linux.
- Full 8-language localization закрита й контролюється CI.
- Planner:
  - Valhalla adapter;
  - trip schema 1.2 derived geometry;
  - real Valhalla HTTP transport;
  - runtime URL / map dataset id / connect + request timeout;
  - `/status` availability + live engine version;
  - `/route` transport без вигаданих vehicle defaults.

## Next
1. Planner live reroute geometry після зміни порядку відкритих точок оператором або водієм.
2. Geometry лишається derived; Control лишається authoritative.
3. Planner desktop release line: Windows + macOS + Linux.
4. Compiled Planner test/stable builds публікувати через GitHub Releases.

Продовження: `START_HERE.md` → private HANDOFF/PROJECT_STATE → Planner STATE/WORKLOG → live GitHub.
