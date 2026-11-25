# Актуальний стан — cargo-dispatch

Оновлено: 2026-11-25, Europe/Kyiv.

Короткий public state; деталі — private `RomanZavadaM/dev-strategy`, branch `profile/cargo-dispatch`.

- Control і Driver Browser інтегровані в `main`: durable trip, route reorder, files/media, location, maps, reconnect.
- Valhalla product adapter та schema 1.2 road-geometry pipeline інтегровані; live external Valhalla HTTP transport ще попереду.
- Android Driver r2 інтегровано: Kotlin/Compose, route actions, foreground location, file/photo/video transfer, restart recovery, 8 мов, Emulator smoke.
- iOS/iPadOS Driver r2 інтегровано: SwiftUI, route actions, CoreLocation, file/photo/video transfer, restart recovery, 8 мов, Simulator smoke.
- Усі driver clients використовують один Control API і не мають окремого source of truth.
- Поточний published prerelease: **v0.1.0-preview.4**; Windows Control asset SHA256 `30028d5fd95653903417411bffbb31c85bf664176092902672224f50e9c0090d`.
- `main` уже випереджає preview.4 native r2 змінами.
- Наступний native крок: MapLibre Android + MapKit Apple, потім connected virtual-device smoke.
- Ще не закрито: formal browser field acceptance, real phone/device LAN, HTTPS/public gateway, browser/native background GPS policy, NAT/CGNAT, live Valhalla transport, production signing/TestFlight, stable release.

Для продовження: `START_HERE.md` → private module logs → live GitHub.
