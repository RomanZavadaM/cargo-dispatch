# cargo-dispatch

**🇺🇦 Українська** · [🇬🇧 English](docs/readme/README.en.md) · [🇫🇷 Français](docs/readme/README.fr.md) · [🇩🇪 Deutsch](docs/readme/README.de.md) · [🇪🇸 Español](docs/readme/README.es.md) · [🇰🇷 한국어](docs/readme/README.ko.md) · [🇯🇵 日本語](docs/readme/README.ja.md) · [🇨🇳 简体中文](docs/readme/README.zh.md)

> **Статус:** активна розробка. Поточний тестовий prerelease — **v0.1.0-preview.3**. Стабільного готового продукту ще немає.

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
- loopback і явний LAN-режим для тестування та локальної роботи.

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

Поточний prerelease: [v0.1.0-preview.3](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.3).

Windows ZIP містить Control, синтетичний активний рейс і тестовий PDF для локального loopback-приймання.

Це **test/prerelease build**, не stable. Важливо: `main` уже випереджає preview.3 — Valhalla adapter і schema 1.2 road-geometry pipeline були інтегровані після публікації цього prerelease.

## Що ще попереду

- ручне local browser acceptance;
- перевірка того самого Driver Browser на реальному телефоні в LAN;
- HTTPS/public addressing path;
- повний browser GPS acceptance через HTTPS;
- польова перевірка через різні мережі/NAT/CGNAT;
- живий Valhalla HTTP transport і перерахунок geometry після reorder;
- native Driver App для Android та iOS/iPadOS — обидва вже розробляються паралельно в окремих draft-гілках;
- stable release.

## Мови інтерфейсу

В інтегрованому desktop/browser UI наразі реалізовано й перевірено **Українську, English і Deutsch**. Для нових native Android та iOS/iPadOS клієнтів мовні пакети закладаються від першого екрана одразу для **uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**. Документація ведеться тим самим набором мов.

## Документація

Короткий публічний стан: [START_HERE.md](START_HERE.md), [HANDOFF_CURRENT.md](HANDOFF_CURRENT.md), [PROJECT_STATE.md](PROJECT_STATE.md).

Розширена робоча документація проєкту ведеться приватно.

## Ліцензія

Власницьке програмне забезпечення © 2026 Roman Zavada (Роман Завада). Публічність репозиторію не надає прав відкритої ліцензії. Див. [LICENSE.md](LICENSE.md) і [COPYRIGHT.md](COPYRIGHT.md).
