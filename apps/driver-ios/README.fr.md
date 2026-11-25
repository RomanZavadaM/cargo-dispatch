# Cargo Dispatch Driver — squelette iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · **🇫🇷 Français** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Premier slice Apple natif. Une seule cible SwiftUI prend en charge iPhone et iPad et utilise **la même API Control** que Driver Browser et l’application Android.

Portée actuelle : Swift 6 + SwiftUI + URLSession, iOS/iPadOS 17+, Driver URL, snapshot du trajet, complete/reorder, identifiants permanents + révision locale, modèle optionnel de géométrie, tests Swift purs et localisations uk/en/de/fr/es/ko/ja/zh-Hans.

Le HTTP local n’est autorisé qu’en debug pour Simulator/LAN ; Release reste HTTPS-first.

Pas encore implémenté : carte native, caméra/photo/vidéo, ouverture/partage de documents, GPS en arrière-plan, file offline, session persistante sécurisée, push, signature/TestFlight.
