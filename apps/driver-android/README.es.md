# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Cliente nativo Android para el conductor. **No sustituye Driver Browser** ni crea un protocolo de servidor separado: utiliza la misma API de Control y las mismas reglas del viaje.

Alcance actual: Kotlin + Jetpack Compose; Android 16/API 36, minSdk 26; Driver URL; snapshot del viaje; complete/reorder; mapa nativo MapLibre con geometría vial, puntos y última posición; ubicación puntual en primer plano; transferencia de archivos/fotos/vídeos y cámara del sistema; cola persistente de reintentos para posiciones y multimedia; seguimiento Start/Stop explícito en segundo plano mediante un servicio de ubicación en primer plano con política de 15 s / 25 m; recuperación segura de sesión, con el token cifrado AES/GCM mediante Android Keystore y solo base URL/modo Emulator no secretos en preferencias privadas; migración automática de la antigua Driver URL sin cifrar tras una reconexión correcta; localización uk/en/de/fr/es/ko/ja/zh-CN; pruebas de protocolo/unitarias y smoke conectado en Emulator contra un viaje sintético real de Control.

HTTP sin cifrar solo en debug local; el camino público es HTTPS-first.

**En el día a día del conductor**

- botón «Abrir en el navegador GPS» en cada punto pendiente;
- «Marcar como hecho» pide confirmación;
- los archivos nuevos se pueden asociar a un punto o a todo el viaje;
- la pantalla abierta se actualiza sola cada 15 segundos e indica si hay conexión;
- sin conexión se guarda el recorrido (hasta las últimas 500 posiciones) y se envía en orden;
- un archivo rechazado por el programa de despacho sigue visible: reenviarlo o quitarlo;
- los mensajes de error aparecen en el idioma del teléfono, sin términos internos.

Pendiente: aceptación LAN/seguimiento en segundo plano en dispositivo real, ruta HTTPS/public/NAT/CGNAT, push/notificaciones y firma de producción.
