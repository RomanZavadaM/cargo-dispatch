# Cargo Dispatch Driver — base Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Primer slice Android nativo. **No sustituye a Driver Browser** ni crea un segundo protocolo de servidor: utiliza la misma API de Control.

Alcance actual: Kotlin + Jetpack Compose, Android 16/API 36, conexión mediante Driver URL, snapshot del viaje activo, complete/reorder, modelo de geometría de ruta, estado de reconexión/error, pruebas de protocolo/unitarias y localizaciones uk/en/de/fr/es/ko/ja/zh-CN.

En Android Emulator, el `127.0.0.1`/localhost del host se mapea a `10.0.2.2`. HTTP sin cifrar solo se permite en debug local; el camino público debe usar HTTPS.

Aún no implementado: cámara/foto/vídeo, abrir/compartir documentos, GPS en segundo plano, cola offline, sesión persistente segura, push y firma de producción.
