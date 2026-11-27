# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · **🇩🇪 Deutsch** · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Nativer Android-Fahrerclient. Er **ersetzt den Driver Browser nicht** und führt kein separates Serverprotokoll ein: Er verwendet dieselbe Control-API und dieselben Tourregeln.

Aktueller Umfang: Kotlin + Jetpack Compose; Android 16/API 36, minSdk 26; Driver-URL; Active-Trip-Snapshot; Complete/Reorder; native MapLibre-Karte mit aktueller Straßengeometrie, Markern und letzter Position; einmalige Vordergrund-Position; Datei/Foto/Video-Transfer und Systemkamera; dauerhafte Retry-Queue für Positionen und Medien; explizites Start/Stop-Hintergrundtracking über einen Foreground-Location-Service mit 15-s-/25-m-Policy; sichere Session-Wiederherstellung mit AES/GCM-Tokenverschlüsselung durch Android Keystore, während nur nicht geheime Base-URL/Emulator-Modus in privaten Preferences liegen; automatische Migration der alten Raw-Driver-URL nach erfolgreichem Reconnect; Lokalisierung uk/en/de/fr/es/ko/ja/zh-CN; Protokoll-/Unit-Tests und connected Emulator-Smoke gegen einen echten synthetischen Control-Trip.

Unverschlüsseltes HTTP ist nur im Debug-Build für lokale Tests erlaubt. Der öffentliche Produktpfad ist HTTPS-first.

**Im Fahreralltag**

- Schaltfläche „Im Navigator öffnen“ an jedem offenen Punkt;
- „Als erledigt markieren“ fragt nach Bestätigung;
- neue Dateien lassen sich einem Punkt oder der ganzen Tour zuordnen;
- der geöffnete Bildschirm aktualisiert sich alle 15 Sekunden und zeigt, ob eine Verbindung besteht;
- ohne Verbindung wird die gefahrene Strecke gespeichert (bis zu 500 letzte Positionen) und der Reihe nach gesendet;
- eine vom Dispositionsprogramm abgelehnte Datei bleibt sichtbar: erneut senden oder entfernen;
- Fehlermeldungen in der Sprache des Telefons, ohne interne Begriffe.

Noch offen: Real-Device-LAN-/Background-Tracking-Abnahme, HTTPS/Public/NAT/CGNAT-Feldpfad, Push/Notifications und Production Signing.
