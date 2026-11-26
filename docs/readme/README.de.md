# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · **🇩🇪 Deutsch** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Status:** aktive Entwicklung. Die aktuelle stabile Version ist **v0.1.0**. Der aktuelle Test-Prerelease ist **v0.1.0-preview.8** und enthält bereits einen separaten Planner für Windows, macOS und Linux. Die vollständige Lokalisierung in 8 Sprachen ist integriert.

## Über das Produkt

**cargo-dispatch** ist ein Dispositionssystem für Frachtflotten. Es soll Ladungen auswählen, Touren planen und neu planen, sie zur Ausführung übergeben und dem Disponenten die Kontrolle über den tatsächlichen Ablauf geben.

Die Hauptentwicklungssprache ist **Rust**.

## Bereits vorhanden

- separate Desktop-Anwendung **Control** auf Basis von Tauri/Leptos; Control kann ein Planner-ZIP importieren oder eine neue aktive Fahrt ohne Planner-ZIP manuell im selben dauerhaften Trip-State anlegen;
- eine separate Desktop-**Planner**-Anwendung auf Tauri/Leptos: manueller Routenentwurf, Punkte hinzufügen/entfernen/neu ordnen, Navigationskoordinaten und Revision über den Planner-Core; der Entwurf wird in einem versionierten Append-only-Journal in den App-Daten gespeichert und nach einem Neustart wiederhergestellt;
- dauerhafte lokale Speicherung der aktiven Tour und Wiederherstellung nach Neustart;
- versioniertes Trip-Paket zwischen Planner und Control;
- ein adaptiver **Driver Browser** für Telefon, Tablet, Laptop und Desktop-Browser;
- Änderung der Reihenfolge offener Stopps in Control und Driver Browser;
- PDF und normale Dateien **Control → Driver Browser**;
- Dateien, Fotos und Videos **Driver Browser → Control** mit lokaler Speicherung;
- zwei gleichzeitige Browser-Clients für dieselbe Tour und Reconnect ohne separate Browser-Datenbank;
- interaktive OpenStreetMap-Karte in Control und Driver Browser;
- Loopback- und expliziter LAN-Modus für Tests und lokale Arbeit.\n- die automatisierte Driver-Browser-Abnahme prüft eine echte Control-Session getrennt über Loopback und einen expliziten privaten LAN-Bind.

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

Control und Driver Browser enthalten bereits eine echte interaktive Basiskarte mit den Fahrtpunkten und der zuletzt übernommenen Fahrerposition.

Der aktuelle `main` verwendet schema 1.4 mit einer unveränderlichen Planungsgrundlage; ältere Schemas bleiben lesbar. Ein Planner-Routingprofil ist nur die Eingabe für eine konkrete Planungsberechnung und keine dauerhafte Bindung einer Fahrt an ein Fahrzeug. Nach dem Import verwaltet Control einen separaten veränderlichen tatsächlichen Routing-Kontext der aktiven Sitzung.

Control speichert abgeleitete Geometrie und berechnet die Straßenlinie nach einer Umordnung über den konfigurierten runtime Valhalla-Dienst neu. Eine verspätete Routing-Antwort kann keinen neueren Fahrtzustand überschreiben. Ist Valhalla nicht konfiguriert oder vorübergehend nicht verfügbar, wird die neue Punktreihenfolge trotzdem gespeichert; die abgeleitete Geometrie kann fehlen.

Planner desktop besitzt jetzt eine eigene runtime Valhalla-Vorschau: Der Operator setzt Valhalla-Adresse, Kartendatensatz-ID und Timeouts, prüft `/status`, und `/route` wird nur mit einem ausdrücklich angegebenen Planungsprofil berechnet. Unbekannte numerische Fahrzeuggrenzen werden nicht erfunden. Ein erfolgreiches Ergebnis zeichnet die echte Straßen-Polyline über OSM; ohne Routenergebnis wird keine künstliche Gerade gezeichnet. Die Vorschau ist von Route/Revision/Profil abgeleitet: Änderungen am Entwurf oder Planungsprofil entfernen die alte Linie, verspätete Antworten werden verworfen. Ein Routing-Fehler verändert den Planner-Entwurf nicht.

Planner desktop exportiert jetzt außerdem das bestehende versionierte Trip-Paket als Schema 1.4 über einen nativen „Speichern unter“-Dialog. Das Paket enthält eine explizite Planungsgrundlage (Fahrzeugtyp/-klasse und Ladungen), die geordnete Punktfolge und – falls vorhanden – nur den passenden Snapshot aus Planning-Routing-Profil und Valhalla-Geometrie. TripId bleibt für einen Entwurf stabil, TripVersion wird über Neustarts fortgeführt, CargoId hängt nicht von der Zeilenposition ab und eine vorhandene ZIP-Datei wird nie überschrieben.

## Test v0.1.0-preview.8

[Test v0.1.0-preview.8 öffnen](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.8).

Dies ist ein **Prerelease**, nicht Stable. Es ist ein integrierter Teststand des aktuellen Produkts aus einer einzigen exakten Quelle.

Der Release enthält:
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

preview.8 enthält bereits den dauerhaften Planner-Entwurf, die echte Valhalla-Straßenvorschau, den versionierten schema-1.4-ZIP-Export mit Planning Basis sowie das manuelle Erstellen einer aktiven Fahrt in Control ohne Planner-ZIP. Das Planner-Profil ist keine dauerhafte Bindung Fahrt→Fahrzeug; der tatsächliche Fahrzeug-/Gerätekontext wird getrennt in Control geführt.

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

- manuelle visuelle lokale Browser-Abnahme für narrow phone / tablet / wide desktop einschließlich Karten-Pan/Zoom/Fit;
- Test desselben Driver Browser auf einem echten Telefon im selben LAN;
- HTTPS-/Public-Addressing-Pfad;
- vollständige Browser-GPS-Abnahme über HTTPS;
- Feldtests über verschiedene Netze/NAT/CGNAT;


## Sprachen der Benutzeroberfläche

Control, Planner, Driver Browser, Android Driver und iOS/iPadOS Driver sind mit demselben **8-Sprachen-Satz: uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN** implementiert und geprüft. Control und Driver Browser verwenden die Systemsprache plus eine gespeicherte manuelle Auswahl; CI prüft die vollständige Abdeckung aller 8 Lokalisierungen.

## Dokumentation

Öffentliche Einstiegspunkte: [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) und [PROJECT_STATE.md](../../PROJECT_STATE.md).

Die ausführliche Arbeitsdokumentation des Projekts wird privat geführt.

## Lizenz

Proprietäre Software © 2026 Roman Zavada. Die öffentliche Sichtbarkeit des Repositorys gewährt keine Open-Source-Rechte. Siehe [LICENSE.md](../../LICENSE.md) und [COPYRIGHT.md](../../COPYRIGHT.md).
