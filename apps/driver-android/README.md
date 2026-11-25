# Cargo Dispatch Driver — Android

**🇺🇦 Українська** · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Нативний Android-клієнт водія. Він **не замінює Driver Browser** і не створює окрему серверну схему: застосунок використовує той самий Control API й ті самі правила рейсу.

## Поточний обсяг

- Kotlin + Jetpack Compose;
- Android 16 / API 36, minSdk 26;
- Driver URL з Control;
- Android Emulator: host `127.0.0.1` / `localhost` автоматично → `10.0.2.2`;
- active trip snapshot, точки, complete та reorder через `local_route_revision`;
- optional road geometry model;
- foreground one-shot GPS → Control;
- PDF/звичайні файли Control → Android з нативним збереженням;
- файл/фото/відео Android → Control через системні pickers;
- native MapLibre карта, current road geometry, точки та latest position;
- системна фото/відео камера;
- durable retry queue для newest location/media;
- explicit Start/Stop background tracking через foreground location service, 15 с / 25 м;
- захищене відновлення сесії: base URL/emulator mode у private preferences, token — AES/GCM через Android Keystore;
- автоматична міграція старого raw Driver URL після успішного reconnect;
- локалізації uk/en/de/fr/es/ko/ja/zh-CN;
- protocol/unit tests + connected Android Emulator smoke проти реального synthetic Control trip.

Debug build дозволяє cleartext HTTP лише для локального тесту. Публічний/production path має бути HTTPS.

## Ще попереду

- real-device LAN/background tracking acceptance;
- HTTPS/public/NAT/CGNAT field path;
- push/notifications;
- production signing/release.
