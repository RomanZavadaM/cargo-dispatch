# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-25, Europe/Kyiv.

Короткий public state; деталі — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Preview.5 release/source commit: `3f3e3897cbd003ff709f588c6351ce5a3f7cfe44`.
- Control і Driver Browser інтегровані: durable trip, route reorder, files/media, location, maps, reconnect.
- Valhalla product adapter та schema 1.2 road-geometry pipeline інтегровані; live external Valhalla HTTP transport ще попереду.
- Android Driver інтегровано через r5: Compose, native MapLibre, camera/files/media, foreground + explicit background tracking, durable retry queue, restart recovery, 8 мов, connected Emulator smoke.
- iOS/iPadOS Driver інтегровано через r5: SwiftUI, native MapKit, camera/files/media, foreground + explicit background CoreLocation tracking, durable retry queue, restart recovery, 8 мов, connected Simulator smoke.
- Android r5 exact-head build/unit + Emulator smoke PASS; Apple r5 core/build/built Info.plist + connected Simulator smoke PASS.
- Усі driver clients використовують один Control API і не мають окремого source of truth.
- Поточний published prerelease: **v0.1.0-preview.5**.
- Windows asset: `Cargo-Dispatch-Control-v0.1.0-preview.5-Windows-x64.zip`, SHA-256 `f0365e86db54a0ee3190bbf60ce8e09a08540649110460ebe7776ac819c0ddfc`.
- Native mobile source входить у source tag; production mobile signing/TestFlight ще попереду.
- Наступний native крок: secure session persistence — Android Keystore / Apple Keychain та міграція legacy raw Driver URL.
- Ще не закрито: formal browser field acceptance, real phone/device LAN, HTTPS/public gateway, NAT/CGNAT field path, live Valhalla transport, production mobile signing/TestFlight, stable release.

Для продовження: `START_HERE.md` → private module logs → live GitHub.
