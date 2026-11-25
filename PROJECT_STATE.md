# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-25, Europe/Kyiv.

Короткий public state; деталі — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Поточний Stable checkpoint: **v0.1.0**.
- Product positioning більше не обмежується малотонажним парком або масою 7,5 т.
- Control desktop підтримується як продукт для Windows, macOS і Linux.
- Planner після появи desktop UI також має виходити для Windows, macOS і Linux.
- Control і Driver Browser інтегровані: durable trip, route reorder, files/media, location, maps, reconnect.
- Addressing Gateway інтегрований: public relay до конкретного Control, RAM-only session mapping, без trip/document/media storage.
- Control має окремий public Gateway mode; inbound-порт на машині Control не потрібен.
- Android Driver: native map, camera/files/media, foreground/background tracking, durable retry queue, secure Keystore session recovery, 8 мов.
- iOS/iPadOS Driver: native map, camera/files/media, foreground/background tracking, durable retry queue, secure Keychain session recovery, 8 мов.
- Release matrix Stable: Control Windows + macOS + Linux, Android Driver, Apple Simulator, Linux Gateway, SHA256SUMS.
- Production mobile signing/App Store/TestFlight/Google Play відкладені на окремий етап.
- Повна локалізація Control, Driver Browser, Android та iOS/iPadOS у 8 мовах інтегрована; CI контролює повноту мовних каталогів і README.
- Наступний великий функціональний етап: **Planner маршруту** — real Valhalla HTTP transport/runtime config, потім live reroute geometry.

Для продовження: `START_HERE.md` → private module logs → live GitHub.
