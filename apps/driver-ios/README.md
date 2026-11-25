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
- відновлення останнього Driver URL після перезапуску;
- локалізований location permission;
- локалізації uk/en/de/fr/es/ko/ja/zh-Hans;
- pure Swift protocol tests + Simulator build/install/launch/screenshot CI.

Debug local HTTP дозволений лише для Simulator/LAN тесту; Release лишається HTTPS-first.

## Ще попереду

- native MapKit map;
- camera capture;
- background GPS;
- connected Simulator smoke проти реального synthetic Control trip;
- offline queue/retry;
- Keychain hardening;
- push/notifications;
- production signing/TestFlight.
