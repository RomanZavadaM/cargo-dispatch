# Cargo Dispatch Driver — Android scaffold

**🇺🇦 Українська** · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Перший native Android slice. Він **не замінює Driver Browser** і не створює нову серверну схему: застосунок працює з тим самим Control API.

## Поточний обсяг

- Kotlin + Jetpack Compose;
- Android 16 / API 36;
- paste Driver URL з Control;
- для Android Emulator автоматична заміна `127.0.0.1` / `localhost` на `10.0.2.2`;
- завантаження active trip snapshot;
- точки, completed state, navigation target;
- optional road geometry model;
- complete point;
- reorder відкритих точок через `local_route_revision`;
- error/reconnect state;
- host-side protocol/unit tests.

## Emulator smoke test

1. Запустити Control на host Windows/Linux/macOS.
2. Підключити active trip і запустити loopback session.
3. Скопіювати повний Driver URL з Control.
4. Запустити Android app в Android Emulator.
5. Лишити прапорець **Android Emulator** увімкненим.
6. Вставити Driver URL і натиснути **Підключити**.
7. Перевірити snapshot, complete та reorder.

Debug build дозволяє cleartext HTTP для локального тесту. Main/release manifest за замовчуванням cleartext забороняє — production path має бути HTTPS.

## Ще не реалізовано

Camera/photo/video, document download/open/share, background GPS, offline queue, push, secure persisted session, production signing і iOS.
