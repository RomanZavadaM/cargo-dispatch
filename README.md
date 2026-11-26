# cargo-dispatch

**🇺🇦 Українська** · [🇬🇧 English](docs/readme/README.en.md) · [🇫🇷 Français](docs/readme/README.fr.md) · [🇩🇪 Deutsch](docs/readme/README.de.md) · [🇪🇸 Español](docs/readme/README.es.md) · [🇰🇷 한국어](docs/readme/README.ko.md) · [🇯🇵 日本語](docs/readme/README.ja.md) · [🇨🇳 简体中文](docs/readme/README.zh.md)

> **Статус:** активна розробка. Поточна стабільна версія — **v0.1.0**. Поточний тестовий prerelease — **v0.1.0-preview.7**, уже з окремим Planner для Windows, macOS і Linux. Повна локалізація 8 мовами інтегрована.

## Про продукт

**cargo-dispatch** — система диспетчеризації вантажного парку. Вона має підбирати вантажі, будувати й перебудовувати рейси, передавати їх у роботу та давати диспетчеру контроль фактичного виконання.

Основна мова розробки — **Rust**.

## Що вже є в поточному продукті

- окремий desktop **Control** на Tauri/Leptos;
- окремий desktop **Planner** на Tauri/Leptos: ручна чернетка маршруту, додавання/видалення/перестановка точок, координати навігації та revision через Planner core; чернетка зберігається у versioned append-only journal в app-data і відновлюється після перезапуску;
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

У поточному `main` використовується schema 1.4 з immutable planning basis; старі schema залишаються читабельними. Planner routing profile — це лише параметри конкретного планового розрахунку, а не постійна прив'язка рейсу до автомобіля. Після імпорту Control веде окремий mutable actual routing context для активної сесії.

Control зберігає derived geometry й після reorder через налаштований runtime Valhalla перераховує дорожню лінію. Запізніла відповідь маршрутизатора не може перезаписати новіший стан рейсу. Якщо Valhalla не налаштована чи тимчасово недоступна, новий порядок точок усе одно зберігається, а derived geometry може бути відсутня.

Planner desktop тепер має власний runtime Valhalla preview: оператор задає адресу Valhalla, ID набору карт і таймаути, перевіряє `/status`, а `/route` рахується тільки з явно заданим planning profile. Невідомі числові параметри не вигадуються. Успішний розрахунок малює реальну дорожню polyline поверх OSM; без результату умовна пряма не малюється. Preview є похідним від route/revision/profile: зміна чернетки або planning profile прибирає стару лінію, а запізніла відповідь не приймається. Невдалий routing не змінює Planner draft.

Planner desktop також уже експортує чинний versioned trip package у schema 1.4 через native Save As. Пакет містить явний planning basis (тип/клас транспорту та вантажі), порядок точок і, за наявності, лише matching snapshot planning routing profile/Valhalla geometry. TripId стабільний для однієї чернетки, TripVersion продовжується між перезапусками, CargoId не залежить від позиції рядка, а існуючий ZIP ніколи не перезаписується.

## Test v0.1.0-preview.7

[Відкрити Test v0.1.0-preview.7](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.7).

Це **prerelease**, не Stable. У ньому вперше опубліковано окремий desktop Planner разом з іншими частинами продукту з одного exact source.

Release містить:
- Windows Control x64 ZIP;
- macOS Control universal ZIP — Intel + Apple Silicon;
- Linux Control x64 tar.gz;
- Windows Planner x64 ZIP;
- macOS Planner universal ZIP — Intel + Apple Silicon;
- Linux Planner x64 tar.gz;
- Android Driver debug APK;
- iOS/iPadOS Driver Simulator ZIP;
- Linux Addressing Gateway x64 package;
- SHA256SUMS.

Перший Planner slice працює з ручною чернеткою маршруту та точками. У preview.7 чернетка Planner ще не зберігається між перезапусками; це наступний етап.

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

- ручне створення нового рейсу безпосередньо в Control, коли Planner ZIP відсутній;

- ручне local browser acceptance;
- перевірка того самого Driver Browser на реальному телефоні в LAN;
- HTTPS/public addressing path;
- повний browser GPS acceptance через HTTPS;
- польова перевірка через різні мережі/NAT/CGNAT;

## Мови інтерфейсу

Control, Planner, Driver Browser, Android Driver та iOS/iPadOS Driver реалізовано й перевірено в одному наборі з **8 мов: uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**. Для Control і Driver Browser є системний вибір мови та ручне перемикання зі збереженням; CI окремо контролює повноту всіх 8 локалізацій.

## Документація

Короткий публічний стан: [START_HERE.md](START_HERE.md), [HANDOFF_CURRENT.md](HANDOFF_CURRENT.md), [PROJECT_STATE.md](PROJECT_STATE.md).

Розширена робоча документація проєкту ведеться приватно.

## Ліцензія

Власницьке програмне забезпечення © 2026 Roman Zavada (Роман Завада). Публічність репозиторію не надає прав відкритої ліцензії. Див. [LICENSE.md](LICENSE.md) і [COPYRIGHT.md](COPYRIGHT.md).
