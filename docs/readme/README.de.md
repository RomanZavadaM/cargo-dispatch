# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · [🇫🇷 Français](README.fr.md) · **🇩🇪 Deutsch** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Disposition für eine Frachtflotte: Tour planen, zur Durchführung übergeben, mit dem Fahrer in Verbindung bleiben und den tatsächlichen Ablauf festhalten. Hauptentwicklungssprache ist **Rust**.

**Stabile Version: [v0.2.0](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.2.0).** Die Entwicklung geht weiter; Testversionen stehen unter [Releases](https://github.com/RomanZavadaM/cargo-dispatch/releases).

## Bestandteile

| Programm | Nutzer | Plattformen |
|---|---|---|
| **Control** | Disponent: führt die aktiven Touren | Windows, macOS, Linux |
| **Planner** | Planer: erstellt Route und Tourpaket | Windows, macOS, Linux |
| **Fahrerseite** | Fahrer in jedem Browser, ohne Installation | Telefon, Tablet, Computer |
| **Driver für Android** | Fahrer | Android 8.0 oder neuer |
| **Driver für iPhone und iPad** | Fahrer | iOS / iPadOS 17 oder neuer |
| **Addressing Gateway** | Verbindung über das Internet | Linux-Server |

Control ist die einzige verbindliche Quelle für den Stand einer aktiven Tour. Es liefert dem Fahrer Seite, Route und Dateien und nimmt Erledigt-Meldungen, Positionen und Anhänge entgegen. Eine zentrale Tourdatenbank gibt es nicht.

## Erste Schritte

1. Archiv von der Release-Seite laden, entpacken und **Control** starten. Das Control-Paket enthält erfundene Testdaten (`local-browser-test-data`).
2. **„Tour hinzufügen“** → „Vorhandene Tour öffnen“ (Tourordner oder Planner-Paket) oder „Neue Tour manuell erstellen“.
3. **„Fahrer verbinden“** → „Dieser Computer“, „Dasselbe WLAN“ oder „Überall, über das Internet“.
4. Dem Fahrer den Link aus dem Block **„Nächste Schritte“** geben: „Öffnen“ oder „Kopieren“.
5. Der Fahrer öffnet den Link im Browser oder fügt ihn in die Driver-App ein.

## Funktionen

**Control**
- mehrere Touren gleichzeitig, eigenes Fenster je Tour;
- Tour aus einem Planner-Paket oder manuell erstellt; der Stand wird lokal gespeichert und nach einem Neustart wiederhergestellt;
- Reihenfolge der offenen Punkte, Karte, letzte Fahrerposition, Vergleich Plan und Ist;
- Dateien an den Fahrer; Dateien, Fotos und Videos vom Fahrer;
- ermittelt die Adresse des Computers im lokalen Netz selbst; die Verbindungsart lässt sich ohne manuelles Abschalten wechseln;
- Fehlermeldungen in der Oberflächensprache; Positionszeit, Dateigröße und Punktname in Klartext.

**Planner**
- Auftragsdatei (JSON, für Mensch und KI) oder Handeingabe; bis zu drei Routenvarianten unter Beachtung der Kapazität, Auswahl und Übernahme in die Tour; Eingaben bleiben erhalten (nach v0.2.0).
- Routenentwurf: Punkte per Koordinaten hinzufügen, entfernen, umstellen; der Entwurf übersteht einen Neustart;
- Routenvorschau und Straßenlinie über Ihren Valhalla-Server mit ausdrücklich eingegebenen Fahrzeugdaten (Unbekanntes wird nicht erfunden);
- Export eines Tourpakets (ZIP, Schema 1.4) mit Ladung und Planungsgrundlage; eine vorhandene Datei wird nie überschrieben.

**Fahrer: Browser, Android, iPhone und iPad**
- Route und Karte, Punkt als erledigt markieren, offene Punkte umstellen;
- Dokumente empfangen; Dateien, Fotos und Videos senden;
- Position übertragen; in den Apps auch im Hintergrund, ausdrücklich ein- und ausschaltbar, mit Warteschlange für erneutes Senden;
- vor dem Erledigt-Markieren wird nachgefragt; die Apps bieten außerdem eine Navigator-Schaltfläche, die Zuordnung einer Datei zu einem Punkt, eine selbstaktualisierende Ansicht, die Speicherung der Strecke ohne Verbindung (bis 500 Positionen) und sichtbare abgelehnte Dateien.

## Verbindung und Sicherheit

- **Dieser Computer** und **lokales Netz** arbeiten unverschlüsselt (HTTP): nur in einem vertrauenswürdigen Netz verwenden. Der Netzzugriff schaltet sich nie selbst ein, Router-Ports werden nicht geöffnet.
- Ein Link mit geheimem Code gibt Zugriff auf die Tour: nur an den Fahrer dieser Tour weitergeben.
- Telefonbrowser erlauben die Standortbestimmung meist nur über HTTPS, also über das Gateway.
- Damit ein Telefon Control im Netz erreicht, das Programm in der Firewall des Computers zulassen.
- **Über das Internet:** eigenes Gateway hinter einem HTTPS-Reverse-Proxy. Control verbindet sich selbst damit, am Computer ist kein eingehender Port nötig. Das Gateway speichert weder Touren noch Dokumente, Medien oder Positionen. Standardgrenze je Anfrage: 64 MiB (bis 512 MiB einstellbar).

## Karten und Routen

Die Karte in Control, Planner, Browser und Android ist [OpenStreetMap](https://www.openstreetmap.org/copyright) (© OpenStreetMap contributors); auf iPhone und iPad ist es Apple MapKit. Der Kartenanbieter sieht die IP-Adresse des Geräts und den angezeigten Ausschnitt. Die Straßenlinie benötigt einen eigenen Valhalla-Server; ohne ihn funktioniert die Tour, nur die Linie fehlt.

## Sprachen

Alle Programme gibt es in українська, English, Français, Deutsch, Español, 한국어, 日本語 und 简体中文. Die CI prüft die Vollständigkeit der Übersetzungen; technische Systemdetails in Fehlermeldungen bleiben englisch.

## Noch nicht geprüft, Einschränkungen

- Die Apps für Android und iPhone/iPad wurden nicht auf echten Geräten getestet – nur im Emulator, im Simulator und mit automatischen Tests.
- Mobiles Internet und das Gateway in einem echten Netz sind nur automatisch geprüft.
- Die Pakete sind nicht signiert; Android ist ein Debug-APK, iOS nur ein Build für den Apple-Simulator.
- Nach einem Neustart von Control sind Touren und Links wieder da; die Verbindung wird mit einem Klick aktiviert (nach v0.2.0).
- Der Disponent kann Punkte noch nicht bearbeiten oder als erledigt markieren.

## Build

Builds, Tests und Releases laufen in GitHub Actions; siehe `.github/workflows`.

## Dokumentation

Projektstand (auf Ukrainisch): [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), [PROJECT_STATE.md](../../PROJECT_STATE.md). Die ausführliche Arbeitsdokumentation wird privat geführt.

## Lizenz

Proprietäre Software © 2026 Roman Zavada (Роман Завада). Ein öffentliches Repository gewährt keine Open-Source-Rechte. Siehe [LICENSE.md](../../LICENSE.md), [COPYRIGHT.md](../../COPYRIGHT.md) und [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md).
