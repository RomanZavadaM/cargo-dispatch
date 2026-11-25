# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · **🇩🇪 Deutsch** · [🇫🇷 Français](README.fr.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Nativer Apple-Fahrerclient. Ein SwiftUI-Target unterstützt iPhone und iPad und verwendet **dieselbe Control-API** wie Driver Browser und Android Driver App.

Aktueller Umfang: Swift 6 + SwiftUI + URLSession; iOS/iPadOS 17+; Driver-URL; Trip-Snapshot; Complete/Reorder; native MapKit-Karte mit aktueller Straßengeometrie, Markern und letzter Position; einmalige CoreLocation-Vordergrundposition; Datei/Foto/Video-Transfer und Systemkamera; dauerhafte Retry-Queue für neueste Position/Medien; explizites Start/Stop-CoreLocation-Tracking im Hintergrund mit `UIBackgroundModes=location` und 25-m-Filter; sichere Session-Wiederherstellung mit Token in einem nicht synchronisierenden Keychain-Eintrag (`AfterFirstUnlockThisDeviceOnly`) und nur der nicht geheimen Base-URL in UserDefaults; automatische Migration der alten Raw-Driver-URL nach erfolgreichem Reconnect; uk/en/de/fr/es/ko/ja/zh-Hans; Swift-Protokolltests und connected Simulator-Smoke gegen einen echten synthetischen Control-Trip.

Lokales HTTP ist nur in Debug für Simulator/LAN erlaubt; Release bleibt HTTPS-first.

Noch offen: Real-Device-LAN-/Background-Tracking-Abnahme, HTTPS/Public/NAT/CGNAT-Feldpfad, Push/Notifications und Production Signing/TestFlight.
