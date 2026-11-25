# HANDOFF_CURRENT — cargo-dispatch

Оновлено: 2026-11-25, Europe/Kyiv.

Це короткий public anchor. Повний стан і журнали — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

## Live checkpoints
- Current product main before preview.5 publication: `99c4654e7183424d28adb72ecbb859c0e3b45bc7`.
- Android Driver r5 merged as `a366f8f8c4ee42dd8202b3dddce7de13948d93d6`.
- Apple Driver r5 exact head `244cb35321189bcf4a0c77c8d35c5d74a772aa51` passed core/build/Info.plist/connected Simulator smoke and merged as `99c4654e7183424d28adb72ecbb859c0e3b45bc7`.
- Test prerelease staged here: `v0.1.0-preview.5`.
- Stable release ще немає.

## Native clients у main
Android та iPhone/iPad використовують той самий Control API, що Driver Browser:
- snapshot, complete, reorder;
- native maps + current road geometry/markers/latest position;
- Control→device documents та device→Control files/photo/video;
- system camera;
- durable retry queue для newest location/media;
- connected Emulator/Simulator smoke проти real synthetic Control session;
- explicit Start/Stop background trip tracking;
- restart recovery;
- 8 мов UI.

Control залишається єдиним джерелом стану рейсу; mobile queues/background tracking не створюють окремої trip database.

## Next
- secure Driver session storage: Android Keystore / Apple Keychain + migration of legacy raw Driver URL;
- real-device LAN/background tracking acceptance;
- HTTPS/public addressing;
- NAT/CGNAT field acceptance;
- live Valhalla HTTP transport/recalculation;
- production mobile signing/TestFlight/release.

Продовження: `START_HERE.md` → private HANDOFF/PROJECT_STATE → active module STATE/WORKLOG → live GitHub.
