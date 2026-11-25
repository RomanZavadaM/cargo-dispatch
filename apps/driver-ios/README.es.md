# Cargo Dispatch Driver — base iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · [🇫🇷 Français](README.fr.md) · **🇪🇸 Español** · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Primer slice nativo de Apple. Un único target SwiftUI funciona en iPhone y iPad y utiliza **la misma API de Control** que Driver Browser y la app Android.

Alcance actual: Swift 6 + SwiftUI + URLSession, iOS/iPadOS 17+, Driver URL, snapshot del viaje, complete/reorder, IDs permanentes + revisión local, modelo opcional de geometría, pruebas puras Swift y localizaciones uk/en/de/fr/es/ko/ja/zh-Hans.

HTTP local solo se permite en debug para Simulator/LAN; Release sigue siendo HTTPS-first.

Aún no implementado: mapa nativo, cámara/foto/vídeo, abrir/compartir documentos, GPS en segundo plano, cola offline, sesión persistente segura, push, firma/TestFlight.
