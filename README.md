# cargo-dispatch

**🇺🇦 Українська** · [🇬🇧 English](docs/readme/README.en.md) · [🇫🇷 Français](docs/readme/README.fr.md) · [🇩🇪 Deutsch](docs/readme/README.de.md) · [🇪🇸 Español](docs/readme/README.es.md) · [🇰🇷 한국어](docs/readme/README.ko.md) · [🇯🇵 日本語](docs/readme/README.ja.md) · [🇨🇳 简体中文](docs/readme/README.zh.md)

> **Статус:** активна розробка. Поточна стабільна версія — **v0.1.0**. Після завершення локалізацій наступний великий напрямок — Planner маршруту.

## Про продукт

**cargo-dispatch** — система диспетчеризації вантажного парку. Вона має підбирати вантажі, будувати й перебудовувати рейси, передавати їх у роботу та давати диспетчеру контроль фактичного виконання.

Основна мова розробки — **Rust**.

## Що вже є в поточному продукті

- окремий desktop **Control** на Tauri/Leptos;
- постійне локальне збереження активного рейсу та відновлення після перезапуску;
- versioned trip package між Planner і Control;
- один адаптивний **Driver Browser** для телефона, планшета та ноутбука/desktop browser;
- зміна порядку відкритих точок у Control і Driver Browser;
- PDF та звичайні файли **Control → Driver Browser**;
- файли, фото й відео **Driver Browser → Control** з локальним збереженням;
- два одночасні browser clients одного рейсу та reconnect без окремої browser-бази;
- інтерактивна карта OpenStreetMap у Control і Driver Browser;
- loopback і явний LAN-режим для тестування та локальної роботи;
- нативний **Android Driver App**: route/complete/reorder, foreground location, files/photo/video, restart recovery, native MapLibre map, system camera, Emulator smoke;
- нативний **iOS/iPadOS Driver App**: route/complete/reorder, CoreLocation foreground location, files/photo/video, restart recovery, native MapKit, system camera, Simulator smoke.

## Нативні Driver Apps

У `main` уже інтегровані обидва нативні клієнти:

- **Android** — Jetpack Compose, route/complete/reorder, files/photo/video, foreground GPS, restart recovery, native MapLibre + OpenStreetMap, current road geometry, route markers/latest position, системна фото/відео камера; Emulator build/install/launch/screenshot PASS.
- **iPhone/iPad** — SwiftUI, route/complete/reorder, files/photo/video, foreground CoreLocation, restart recovery, native MapKit, current road geometry, route markers/latest position, системна фото/відео камера; Simulator build/install/launch/screenshot PASS.

Обидва працюють через той самий Control API і не мають окремої бази рейсу.

Після r4/r5 обидва native clients також мають надійну чергу повторної відправки координат і медіа. Android r5 має явний Start/Stop фонового відстеження через foreground location service, постійне повідомлення зі Stop і політику 15 с / 25 м. Apple r5 має явний Start/Stop фонового CoreLocation tracking, `UIBackgroundModes=location`, фільтр 25 м і не просить `Always` без окремої потреби.

## Driver Browser

Driver Browser — **один продукт**, а не окремі телефонна й desktop-версії. Він має три штатні компонування:

- narrow phone;
- tablet;
- wide desktop/laptop.

Дані, маршрут, файли та backend однакові. Змінюється лише компонування під розмір екрана.

## Карта

У Control і Driver Browser вже є справжня інтерактивна базова карта з точками рейсу та останньою прийнятою позицією водія.

У `main` уже інтегровано versioned road-geometry pipeline (schema 1.2): Planner може передати похідну геометрію, Control зберігає її, обидві карти показують дорожню лінію лише поки вона відповідає поточному порядку точок. Після reorder стара лінія негайно інвалідовується.

Ще попереду **живий HTTP transport до Valhalla та автоматичний перерахунок geometry**. Тобто структура й відображення дороги вже є, але зовнішній маршрутизатор ще не підключений до product runtime.

## Stable v0.1.0

[Відкрити Stable v0.1.0](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0).

Stable збирається з одного exact source і публікує:
- Windows Control x64 ZIP;
- macOS Control universal ZIP — Intel + Apple Silicon;
- Linux Control x64 tar.gz;
- Android Driver debug APK для тестового встановлення;
- iOS/iPadOS Driver Simulator ZIP;
- Linux Addressing Gateway x64 package з Docker/Compose;
- SHA256SUMS.

**Linux Control** — desktop-застосунок оператора. **Linux Addressing Gateway** — окремий серверний relay-модуль; це не один і той самий пакет.

Android та Apple пакети поки не підписані для магазинів. App Store/TestFlight/Google Play і production signing опрацюємо окремо пізніше.

## Що ще попереду

- ручне local browser acceptance;
- перевірка того самого Driver Browser на реальному телефоні в LAN;
- HTTPS/public addressing path;
- повний browser GPS acceptance через HTTPS;
- польова перевірка через різні мережі/NAT/CGNAT;
- живий Valhalla HTTP transport і перерахунок geometry після reorder;
- захищене збереження Driver session: Android Keystore + Apple Keychain і міграція старого raw URL;
- stable release.

## Мови інтерфейсу

В інтегрованому desktop/browser UI наразі реалізовано й перевірено **Українську, English і Deutsch**. Для нових native Android та iOS/iPadOS клієнтів мовні пакети закладаються від першого екрана одразу для **uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**. Документація ведеться тим самим набором мов.

## Документація

Короткий публічний стан: [START_HERE.md](START_HERE.md), [HANDOFF_CURRENT.md](HANDOFF_CURRENT.md), [PROJECT_STATE.md](PROJECT_STATE.md).

Розширена робоча документація проєкту ведеться приватно.

## Ліцензія

Власницьке програмне забезпечення © 2026 Roman Zavada (Роман Завада). Публічність репозиторію не надає прав відкритої ліцензії. Див. [LICENSE.md](LICENSE.md) і [COPYRIGHT.md](COPYRIGHT.md).
