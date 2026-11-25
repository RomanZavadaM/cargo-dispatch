# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · **🇩🇪 Deutsch** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Status:** aktive Entwicklung. Der aktuelle Test-Prerelease ist **v0.1.0-preview.4**. Ein stabiles fertiges Produkt gibt es noch nicht.

## Über das Produkt

**cargo-dispatch** ist ein Dispositionssystem für eine Flotte kleiner Lkw (zulässiges Gesamtgewicht bis 7,5 t). Es soll Ladungen auswählen, Touren planen und neu planen, sie zur Ausführung übergeben und dem Disponenten die Kontrolle über den tatsächlichen Ablauf geben.

Die Hauptentwicklungssprache ist **Rust**.

## Bereits vorhanden

- separate Desktop-Anwendung **Control** auf Basis von Tauri/Leptos;
- dauerhafte lokale Speicherung der aktiven Tour und Wiederherstellung nach Neustart;
- versioniertes Trip-Paket zwischen Planner und Control;
- ein adaptiver **Driver Browser** für Telefon, Tablet, Laptop und Desktop-Browser;
- Änderung der Reihenfolge offener Stopps in Control und Driver Browser;
- PDF und normale Dateien **Control → Driver Browser**;
- Dateien, Fotos und Videos **Driver Browser → Control** mit lokaler Speicherung;
- zwei gleichzeitige Browser-Clients für dieselbe Tour und Reconnect ohne separate Browser-Datenbank;
- interaktive OpenStreetMap-Karte in Control und Driver Browser;
- Loopback- und expliziter LAN-Modus für Tests und lokale Arbeit.

## Native Driver Apps

Beide nativen Clients sind bereits in `main` integriert:

- **Android** — Jetpack Compose, Route/Abschluss/Neuordnung, Dateien/Fotos/Videos, Foreground-GPS, Session-Wiederherstellung, native MapLibre + OpenStreetMap, aktuelle Straßengeometrie, Stopps/letzte Position und systemeigene Foto-/Videoaufnahme; Emulator Build/Install/Start/Screenshot PASS.
- **iPhone/iPad** — SwiftUI, Route/Abschluss/Neuordnung, Dateien/Fotos/Videos, Foreground-CoreLocation, Session-Wiederherstellung, natives MapKit, aktuelle Geometrie, Stopps/letzte Position und systemeigene Foto-/Videoaufnahme; Simulator Build/Install/Start/Screenshot PASS.

Beide verwenden dieselbe Control-API und erzeugen keine separate Tour-Datenbank.

## Driver Browser

Driver Browser ist **ein einziges Produkt**, nicht getrennte Telefon- und Desktop-Versionen. Es gibt drei reguläre Layouts:

- schmales Telefon;
- Tablet;
- breiter Desktop/Laptop.

Daten, Routenaktionen, Dateien und Backend sind gemeinsam; nur das Layout passt sich der Bildschirmgröße an.

## Karte

Control und Driver Browser enthalten jetzt eine echte interaktive Basiskarte mit den Tourpunkten und der zuletzt akzeptierten Fahrerposition.

In `main` ist jetzt eine versionierte Road-Geometry-Pipeline (Schema 1.2) integriert: Planner kann abgeleitete Geometrie mitgeben, Control speichert sie, und beide Karten zeigen die Straßenlinie nur solange sie zur aktuellen Stopp-Reihenfolge passt. Nach einem Reorder wird die alte Linie sofort ungültig.

Noch fehlt der **Live-HTTP-Transport zu Valhalla und die automatische Neuberechnung der Geometrie**. Vertrag und Darstellung sind vorhanden; der externe Routing-Engine-Pfad ist noch nicht in die Product-Runtime eingebunden.

## Testversion

Aktueller Prerelease: [v0.1.0-preview.4](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.4).

Das Windows-ZIP enthält Control, eine synthetische aktive Tour und eine Test-PDF für die lokale Loopback-Abnahme.

Dies ist ein **Test-/Prerelease-Build**, keine stabile Version. Preview.4 enthält bereits Valhalla-Adapter und Schema-1.2-Road-Geometry-Pipeline. `main` ist erneut weiter: Android- und iOS/iPadOS-Driver r2+r3 einschließlich nativer Karten und Systemkamera sowie ein verbesserter Driver-URL-Handoff aus Control wurden nach preview.4 integriert.

## Was noch fehlt

- manuelle lokale Browser-Abnahme;
- Test desselben Driver Browser auf einem echten Telefon im selben LAN;
- HTTPS-/Public-Addressing-Pfad;
- vollständige Browser-GPS-Abnahme über HTTPS;
- Feldtests über verschiedene Netze/NAT/CGNAT;
- Live-Valhalla-HTTP-Transport und Neuberechnung der Geometrie nach Reorder;
- verbundener Virtual-Device-Smoke über eine echte headless Control-Session;
- Hintergrund-GPS, Offline-Queue und sichere Session-Speicherung für die nativen Apps;
- Stable Release.

## Sprachen der Benutzeroberfläche

Die integrierte Desktop-/Browser-Oberfläche implementiert und prüft derzeit **Ukrainisch, Englisch und Deutsch**. Die neuen nativen Android- und iOS/iPadOS-Clients erhalten von Anfang an Sprachpakete für **uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**; die Dokumentation wird mit demselben Sprachsatz gepflegt.

## Dokumentation

Öffentliche Einstiegspunkte: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) und [PROJECT_STATE.md](../../PROJECT_STATE.md).

Die ausführliche Arbeitsdokumentation des Projekts wird privat geführt.

## Lizenz

Proprietäre Software © 2026 Roman Zavada. Die öffentliche Sichtbarkeit des Repositorys gewährt keine Open-Source-Rechte. Siehe [LICENSE.md](../../LICENSE.md) und [COPYRIGHT.md](../../COPYRIGHT.md).
