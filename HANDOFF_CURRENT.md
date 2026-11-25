# HANDOFF_CURRENT — cargo-dispatch

Оновлено: 2026-11-25, Europe/Kyiv.

Це короткий public anchor. Повний стан і журнали — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

## Live checkpoints
- Android Driver r2 merged: `41b3dcdbfbfa8ae0897291e4fb736e244649f1e3`.
- iOS/iPadOS Driver r2 merged: `b37c21e68b25fbaf8a4373be3c169204acca16d9`.
- Android exact-head CI перед merge: build/unit tests + Emulator install/launch/screenshot **SUCCESS**.
- Apple exact-head code/docs `15f403a70ba12fee06bc0b2fa1bd3916518735da`: core tests + Simulator build/install/launch/screenshot **SUCCESS**.
- Published prerelease: `v0.1.0-preview.4`.
- Stable release ще немає.

## Native clients у main
Android та iPhone/iPad використовують той самий Control API, що Driver Browser:
- snapshot, complete, reorder;
- foreground location;
- Control→device document download/save;
- device→Control file/photo/video upload;
- restart recovery останньої Driver connection;
- 8 мов UI.

## Next
- Android native map через MapLibre;
- Apple native map через MapKit;
- connected virtual-device smoke проти synthetic real Control trip;
- camera capture, background GPS, offline/security;
- live Valhalla HTTP transport;
- HTTPS/public/NAT/CGNAT acceptance.

Продовження: `START_HERE.md` → private HANDOFF/PROJECT_STATE → active module STATE/WORKLOG → live GitHub.
