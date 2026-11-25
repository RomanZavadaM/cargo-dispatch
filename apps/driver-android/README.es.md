# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Cliente nativo Android para el conductor. **No sustituye Driver Browser** ni crea un protocolo de servidor separado: utiliza la misma API de Control y las mismas reglas del viaje.

Alcance actual: Kotlin + Jetpack Compose; Android 16/API 36, minSdk 26; Driver URL; loopback del Emulator mediante `10.0.2.2`; snapshot del viaje; complete/reorder; geometría de ruta opcional; ubicación puntual en primer plano hacia Control; descarga Control→Android con guardado nativo; envío de archivo/foto/vídeo Android→Control mediante selectores del sistema; recuperación de la última Driver URL/modo Emulator; localización uk/en/de/fr/es/ko/ja/zh-CN; pruebas de protocolo/unitarias y CI de instalación/arranque/captura en Emulator.

HTTP sin cifrar solo en debug local; el camino público es HTTPS-first.

Pendiente: mapa nativo, captura de cámara, GPS en segundo plano, smoke conectado a un viaje sintético real de Control, cola offline/reintentos, endurecimiento de Keystore, push y firma de producción.
