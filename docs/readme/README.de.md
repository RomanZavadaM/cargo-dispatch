# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · **🇩🇪 Deutsch** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Status:** aktive Entwicklung. Die aktuelle stabile Version ist **v0.1.0**. Die vollständige Lokalisierung in 8 Sprachen ist integriert; der nächste große Schwerpunkt ist der Routen-Planner.

## Über das Produkt

**cargo-dispatch** ist ein Dispositionssystem für Frachtflotten. Es soll Ladungen auswählen, Touren planen und neu planen, sie zur Ausführung übergeben und dem Disponenten die Kontrolle über den tatsächlichen Ablauf geben.

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

Seit r4/r5 besitzen beide nativen Clients außerdem eine dauerhafte Retry-Queue für Positionen und Medien. Android r5 ergänzt explizites Start/Stop-Hintergrundtracking über einen Foreground-Location-Service, eine dauerhafte Stop-Benachrichtigung und eine 15-s-/25-m-Policy. Apple r5 ergänzt explizites Start/Stop-CoreLocation-Tracking im Hintergrund, `UIBackgroundModes=location`, einen 25-m-Filter und fordert `Always` nicht ohne gesonderten Bedarf an.

## Driver Browser

Driver Browser ist **ein einziges Produkt**, nicht getrennte Telefon- und Desktop-Versionen. Es gibt drei reguläre Layouts:

- schmales Telefon;
- Tablet;
- breiter Desktop/Laptop.

Daten, Routenaktionen, Dateien und Backend sind gemeinsam; nur das Layout passt sich der Bildschirmgröße an.

## Karte

Control und Driver Browser enthalten jetzt eine echte interaktive Basiskarte mit den Tourpunkten und der zuletzt akzeptierten Fahrerposition.

In `main` ist jetzt eine versionierte Road-Geometry-Pipeline (Schema 1.3) integriert: Planner übergibt abgeleitete Geometrie zusammen mit dem tatsächlichen Fahrzeug-Routingprofil; Pakete der Schemas 1.0–1.2 bleiben lesbar. Control speichert die Geometrie und berechnet nach einem Reorder die Straßenlinie über den konfigurierten Valhalla-Runtime-Dienst neu. Eine verspätete Routing-Antwort kann keinen neueren Tourzustand überschreiben.

Valhalla-Adresse, Karten-Dataset-ID und Timeouts werden direkt in Desktop Control eingestellt; die Verfügbarkeit lässt sich über `/status` prüfen oder das Routing ausdrücklich deaktivieren. Ist Valhalla nicht konfiguriert oder vorübergehend nicht verfügbar, speichert Control die neue Punktreihenfolge trotzdem; die abgeleitete Geometrie kann dann fehlen.

## Stable v0.1.0

[Stable v0.1.0 öffnen](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0).

Stable wird aus genau einem Quellstand neu gebaut und veröffentlicht:
- Windows Control x64 ZIP;
- macOS Control universal ZIP — Intel + Apple Silicon;
- Linux Control x64 tar.gz;
- Android Driver debug APK für Testinstallation;
- iOS/iPadOS Driver Simulator ZIP;
- Linux Addressing Gateway x64 Paket mit Docker/Compose;
- SHA256SUMS.

**Linux Control** ist die Desktop-Anwendung des Operators. **Linux Addressing Gateway** ist das separate Server-Relay-Modul; es sind zwei verschiedene Pakete.

Android- und Apple-Pakete sind noch nicht für die Stores signiert. App Store/TestFlight/Google Play und Production Signing folgen später separat.

## Was noch fehlt

- Planner/editor-Desktop-UI und Release-Builds für Windows, macOS und Linux;

- manuelle lokale Browser-Abnahme;
- Test desselben Driver Browser auf einem echten Telefon im selben LAN;
- HTTPS-/Public-Addressing-Pfad;
- vollständige Browser-GPS-Abnahme über HTTPS;
- Feldtests über verschiedene Netze/NAT/CGNAT;


## Sprachen der Benutzeroberfläche

Control, Driver Browser, Android Driver und iOS/iPadOS Driver sind mit demselben **8-Sprachen-Satz: uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN** implementiert und geprüft. Control und Driver Browser verwenden die Systemsprache plus eine gespeicherte manuelle Auswahl; CI prüft die vollständige Abdeckung aller 8 Lokalisierungen.

## Dokumentation

Öffentliche Einstiegspunkte: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) und [PROJECT_STATE.md](../../PROJECT_STATE.md).

Die ausführliche Arbeitsdokumentation des Projekts wird privat geführt.

## Lizenz

Proprietäre Software © 2026 Roman Zavada. Die öffentliche Sichtbarkeit des Repositorys gewährt keine Open-Source-Rechte. Siehe [LICENSE.md](../../LICENSE.md) und [COPYRIGHT.md](../../COPYRIGHT.md).
