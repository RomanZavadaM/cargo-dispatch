# HANDOFF_CURRENT — cargo-dispatch

Оновлено: 2026-11-25, Europe/Kyiv.

Це короткий public anchor. Повний стан і журнали — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

## Stable checkpoint
- Поточна стабільна версія: `v0.1.0`.
- Exact source і SHA256 пакетів фіксуються в самому GitHub Release.
- Release matrix:
  - Control Windows x64;
  - Control macOS universal — Intel + Apple Silicon;
  - Control Linux x64;
  - Driver Android debug APK;
  - Driver iOS/iPadOS Simulator ZIP;
  - Addressing Gateway Linux x64 + Docker/Compose;
  - спільний `SHA256SUMS.txt`.
- Mobile store signing, TestFlight і Google Play поки свідомо відкладені.

## Інтегровано
- Control + Driver Browser з durable trip state, maps, files/media, location, route reorder і reconnect.
- Android Driver із MapLibre, camera/files, durable retry queue, explicit background tracking і Keystore-protected session.
- Apple Driver із MapKit, camera/files, durable retry queue, explicit background tracking і Keychain-protected session.
- Addressing Gateway: RAM-only relay, Docker deployment, без центрального зберігання рейсу.
- Desktop Control має режим `Публічно через Gateway` і показує окремий public Driver URL.
- Control desktop release line: Windows + macOS + Linux.

## Next
1. Закрити повну локалізацію Control і Driver Browser усіма мовами проєкту.
2. Звірити README/мобільні описи після локалізації.
3. Перейти до Planner маршруту.
4. Planner desktop release line: Windows + macOS + Linux.

Продовження: `START_HERE.md` → private HANDOFF/PROJECT_STATE → active module STATE/WORKLOG → live GitHub.
