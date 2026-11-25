# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Cliente nativo Apple para el conductor. Un único target SwiftUI funciona en iPhone y iPad y utiliza **la misma API de Control** que Driver Browser y la app Android.

Alcance actual: Swift 6 + SwiftUI + URLSession; iOS/iPadOS 17+; Driver URL; snapshot; complete/reorder; mapa MapKit nativo con geometría vial, puntos y última posición; CoreLocation puntual en primer plano; transferencia de archivos/fotos/vídeos y cámara del sistema; cola persistente de reintentos para última ubicación/multimedia; seguimiento CoreLocation Start/Stop explícito en segundo plano con `UIBackgroundModes=location` y filtro de 25 m; recuperación segura de sesión con el token en un elemento Keychain no sincronizado (`AfterFirstUnlockThisDeviceOnly`) y solo la base URL no secreta en UserDefaults; migración automática de la antigua Driver URL sin cifrar tras reconexión correcta; uk/en/de/fr/es/ko/ja/zh-Hans; pruebas de protocolo Swift y smoke conectado en Simulator contra un viaje sintético real de Control.

HTTP local solo en Debug Simulator/LAN; Release sigue siendo HTTPS-first.

Pendiente: aceptación LAN/seguimiento en segundo plano en dispositivo real, ruta HTTPS/public/NAT/CGNAT, push/notificaciones y firma de producción/TestFlight.
