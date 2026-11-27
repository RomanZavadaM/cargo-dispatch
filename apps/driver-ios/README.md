# Cargo Dispatch Driver — iOS/iPadOS

**🇺🇦 Українська** · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Нативний Apple-клієнт водія. Один SwiftUI target працює на iPhone та iPad і використовує **той самий Control API**, що Driver Browser та Android Driver App.

## Поточний обсяг

- Swift 6 + SwiftUI + URLSession;
- deployment target iOS/iPadOS 17+;
- Driver URL з Control;
- trip snapshot, complete/reorder, permanent point IDs + local route revision;
- optional road geometry model;
- CoreLocation one-shot foreground GPS → Control;
- Control → iPhone/iPad document download через native fileExporter;
- iPhone/iPad → Control file/photo/video через fileImporter + PhotosPicker;
- native MapKit map з current road geometry, точками та latest position;
- системна camera capture для фото/відео;
- черга повторного надсилання позицій і файлів;
- explicit Start/Stop background CoreLocation tracking, `UIBackgroundModes=location`, 25 м filter;
- захищене відновлення сесії: token у non-synchronizing Keychain (`AfterFirstUnlockThisDeviceOnly`), base URL у UserDefaults;
- автоматична міграція старого raw Driver URL після успішного reconnect;
- локалізований location permission;
- локалізації uk/en/de/fr/es/ko/ja/zh-Hans;
- pure Swift protocol tests + connected Simulator smoke проти real synthetic Control trip.

Debug local HTTP дозволений лише для Simulator/LAN тесту; Release лишається HTTPS-first.

## У щоденній роботі водія

- кнопка «Відкрити в навігаторі» біля невиконаної точки;
- «Позначити виконаною» з підтвердженням;
- нові файли можна прив'язати до точки або до всього рейсу;
- екран оновлюється сам кожні 15 секунд, поки відкритий, і показує, чи є зв'язок;
- без зв'язку зберігається пройдений шлях (до 500 останніх позицій) і надсилається по порядку;
- файл, який диспетчерська програма не прийняла, лишається видимим: надіслати ще раз або прибрати;
- повідомлення про помилки — мовою телефона, без службових слів.

## Ще попереду

- real-device LAN/background tracking acceptance;
- HTTPS/public/NAT/CGNAT field path;
- push/notifications;
- production signing/TestFlight.
