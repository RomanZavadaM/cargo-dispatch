# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · **🇩🇪 Deutsch** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Status:** aktive Entwicklung. Der aktuelle Test-Prerelease ist **v0.1.0-preview.3**. Ein stabiles fertiges Produkt gibt es noch nicht.

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

## Driver Browser

Driver Browser ist **ein einziges Produkt**, nicht getrennte Telefon- und Desktop-Versionen. Es gibt drei reguläre Layouts:

- schmales Telefon;
- Tablet;
- breiter Desktop/Laptop.

Daten, Routenaktionen, Dateien und Backend sind gemeinsam; nur das Layout passt sich der Bildschirmgröße an.

## Karte

Control und Driver Browser enthalten jetzt eine echte interaktive Basiskarte mit den Tourpunkten und der zuletzt akzeptierten Fahrerposition.

Dies ist **noch nicht die vollständige Straßenroutengeometrie**. Die reale Geometrie über Valhalla muss noch integriert werden.

## Testversion

Aktueller Prerelease: [v0.1.0-preview.3](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.3).

Das Windows-ZIP enthält Control, eine synthetische aktive Tour und eine Test-PDF für die lokale Loopback-Abnahme.

Dies ist ein **Test-/Prerelease-Build**, keine stabile Version.

## Was noch fehlt

- manuelle lokale Browser-Abnahme;
- Test desselben Driver Browser auf einem echten Telefon im selben LAN;
- HTTPS-/Public-Addressing-Pfad;
- vollständige Browser-GPS-Abnahme über HTTPS;
- Feldtests über verschiedene Netze/NAT/CGNAT;
- reale Valhalla-Straßengeometrie;
- zukünftige native Driver App;
- Stable Release.

## Sprachen der Benutzeroberfläche

Die aktuelle Oberfläche implementiert und prüft **Ukrainisch, Englisch und Deutsch**. Die übrigen README-Übersetzungen sind Dokumentations- bzw. Zielsprachen und bedeuten noch keine Unterstützung in der Programmoberfläche.

## Dokumentation

Öffentliche Einstiegspunkte: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) und [PROJECT_STATE.md](../../PROJECT_STATE.md).

Die ausführliche Arbeitsdokumentation des Projekts wird privat geführt.

## Lizenz

Proprietäre Software © 2026 Roman Zavada. Die öffentliche Sichtbarkeit des Repositorys gewährt keine Open-Source-Rechte. Siehe [LICENSE.md](../../LICENSE.md) und [COPYRIGHT.md](../../COPYRIGHT.md).
