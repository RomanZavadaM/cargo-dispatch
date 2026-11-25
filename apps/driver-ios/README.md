# Cargo Dispatch Driver — iOS/iPadOS scaffold

Перший native Apple slice. Один SwiftUI app target працює на iPhone та iPad і використовує **той самий Control API**, що Driver Browser та Android Driver App.

## Поточний обсяг

- Swift 6 + SwiftUI + URLSession;
- deployment target iOS/iPadOS 17+;
- Driver URL з Control;
- trip snapshot;
- complete/reorder;
- permanent point IDs + local route revision;
- optional road geometry model;
- shared pure Swift core + Swift Testing;
- String Catalog одразу для uk/en/de/fr/es/ko/ja/zh-Hans;
- Debug local HTTP дозволений лише для локального Simulator/LAN тесту; Release лишається HTTPS-first.

## Simulator smoke test

1. Запустити Control на Mac.
2. Підключити active trip і запустити loopback session.
3. Відкрити iOS Simulator.
4. Запустити CargoDispatchDriver.
5. Вставити Driver URL з Control.
6. Перевірити snapshot, complete та reorder.

На iOS Simulator loopback host не потребує Android-перетворення на 10.0.2.2.

## Ще не реалізовано

Native map rendering, camera/photo/video, document open/share, background GPS, offline queue, secure persisted session, push, production signing/TestFlight.
