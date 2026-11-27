# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Cliente nativo Apple para el conductor. Un único target SwiftUI funciona en iPhone y iPad y utiliza **la misma API de Control** que Driver Browser y la app Android.

Alcance actual: Swift 6 + SwiftUI + URLSession; iOS/iPadOS 17+; Driver URL; snapshot; complete/reorder; mapa MapKit nativo con geometría vial, puntos y última posición; CoreLocation puntual en primer plano; transferencia de archivos/fotos/vídeos y cámara del sistema; cola persistente de reintentos para posiciones y multimedia; seguimiento CoreLocation Start/Stop explícito en segundo plano con `UIBackgroundModes=location` y filtro de 25 m; recuperación segura de sesión con el token en un elemento Keychain no sincronizado (`AfterFirstUnlockThisDeviceOnly`) y solo la base URL no secreta en UserDefaults; migración automática de la antigua Driver URL sin cifrar tras reconexión correcta; uk/en/de/fr/es/ko/ja/zh-Hans; pruebas de protocolo Swift y smoke conectado en Simulator contra un viaje sintético real de Control.

HTTP local solo en Debug Simulator/LAN; Release sigue siendo HTTPS-first.

**En el día a día del conductor**

- botón «Abrir en el navegador GPS» en cada punto pendiente;
- «Marcar como hecho» pide confirmación;
- los archivos nuevos se pueden asociar a un punto o a todo el viaje;
- la pantalla abierta se actualiza sola cada 15 segundos e indica si hay conexión;
- sin conexión se guarda el recorrido (hasta las últimas 500 posiciones) y se envía en orden;
- un archivo rechazado por el programa de despacho sigue visible: reenviarlo o quitarlo;
- los mensajes de error aparecen en el idioma del teléfono, sin términos internos.

Pendiente: aceptación LAN/seguimiento en segundo plano en dispositivo real, ruta HTTPS/public/NAT/CGNAT, push/notificaciones y firma de producción/TestFlight.
