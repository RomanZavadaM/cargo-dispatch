# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Cliente nativo Apple para el conductor. Un único target SwiftUI funciona en iPhone y iPad y utiliza **la misma API de Control** que Driver Browser y la app Android.

Alcance actual: Swift 6 + SwiftUI + URLSession; iOS/iPadOS 17+; Driver URL; snapshot; complete/reorder; IDs permanentes + revisión local; geometría opcional; ubicación puntual CoreLocation en primer plano hacia Control; descarga Control→iPhone/iPad mediante fileExporter; archivo/foto/vídeo iPhone/iPad→Control mediante fileImporter + PhotosPicker; recuperación de la última Driver URL; permiso de ubicación localizado; uk/en/de/fr/es/ko/ja/zh-Hans; pruebas de protocolo Swift y CI Simulator build/install/launch/screenshot.

HTTP local solo en Debug Simulator/LAN; Release sigue siendo HTTPS-first.

Pendiente: mapa MapKit nativo, captura de cámara, GPS en segundo plano, smoke conectado a un viaje sintético real de Control, cola offline/reintentos, endurecimiento de Keychain, push y firma/TestFlight.
