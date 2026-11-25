# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Cliente nativo Android para el conductor. **No sustituye Driver Browser** ni crea un protocolo de servidor separado: utiliza la misma API de Control y las mismas reglas del viaje.

Alcance actual: Kotlin + Jetpack Compose; Android 16/API 36, minSdk 26; Driver URL; snapshot del viaje; complete/reorder; mapa nativo MapLibre con geometría vial, puntos y última posición; ubicación puntual en primer plano; transferencia de archivos/fotos/vídeos y cámara del sistema; cola persistente de reintentos para la última ubicación/multimedia; seguimiento Start/Stop explícito en segundo plano mediante un servicio de ubicación en primer plano con política de 15 s / 25 m; recuperación segura de sesión, con el token cifrado AES/GCM mediante Android Keystore y solo base URL/modo Emulator no secretos en preferencias privadas; migración automática de la antigua Driver URL sin cifrar tras una reconexión correcta; localización uk/en/de/fr/es/ko/ja/zh-CN; pruebas de protocolo/unitarias y smoke conectado en Emulator contra un viaje sintético real de Control.

HTTP sin cifrar solo en debug local; el camino público es HTTPS-first.

Pendiente: aceptación LAN/seguimiento en segundo plano en dispositivo real, ruta HTTPS/public/NAT/CGNAT, push/notificaciones y firma de producción.
