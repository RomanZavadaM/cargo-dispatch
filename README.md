# cargo-dispatch

**🇺🇦 Українська** · [🇬🇧 English](docs/readme/README.en.md) · [🇫🇷 Français](docs/readme/README.fr.md) · [🇩🇪 Deutsch](docs/readme/README.de.md) · [🇪🇸 Español](docs/readme/README.es.md) · [🇰🇷 한국어](docs/readme/README.ko.md) · [🇯🇵 日本語](docs/readme/README.ja.md) · [🇨🇳 简体中文](docs/readme/README.zh.md)

> **Статус:** активна розробка. Поточний тестовий prerelease — **v0.1.0-preview.4**. Стабільного готового продукту ще немає.

## Про продукт

**cargo-dispatch** — система диспетчеризації для малотонажного вантажного парку (повна маса до 7,5 т). Вона має підбирати вантажі, будувати й перебудовувати рейси, передавати їх у роботу та давати диспетчеру контроль фактичного виконання.

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

## Тестова збірка

Поточний prerelease: [v0.1.0-preview.4](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.4).

Windows ZIP містить Control, синтетичний активний рейс і тестовий PDF для локального loopback-приймання.

Це **test/prerelease build**, не stable. Preview.4 уже містить Valhalla adapter і schema 1.2 road-geometry pipeline. `main` знову випереджає preview.4: після його публікації інтегровано Android та iOS/iPadOS native Driver r2+r3, включно з native maps і системною камерою, а також покращено передачу Driver URL з Control.

## Що ще попереду

- ручне local browser acceptance;
- перевірка того самого Driver Browser на реальному телефоні в LAN;
- HTTPS/public addressing path;
- повний browser GPS acceptance через HTTPS;
- польова перевірка через різні мережі/NAT/CGNAT;
- живий Valhalla HTTP transport і перерахунок geometry після reorder;
- connected virtual-device smoke через реальний headless Control session;
- background GPS, offline queue та secure session storage для native apps;
- stable release.

## Мови інтерфейсу

В інтегрованому desktop/browser UI наразі реалізовано й перевірено **Українську, English і Deutsch**. Для нових native Android та iOS/iPadOS клієнтів мовні пакети закладаються від першого екрана одразу для **uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**. Документація ведеться тим самим набором мов.

## Документація

Короткий публічний стан: [START_HERE.md](START_HERE.md), [HANDOFF_CURRENT.md](HANDOFF_CURRENT.md), [PROJECT_STATE.md](PROJECT_STATE.md).

Розширена робоча документація проєкту ведеться приватно.

## Ліцензія

Власницьке програмне забезпечення © 2026 Roman Zavada (Роман Завада). Публічність репозиторію не надає прав відкритої ліцензії. Див. [LICENSE.md](LICENSE.md) і [COPYRIGHT.md](COPYRIGHT.md).
