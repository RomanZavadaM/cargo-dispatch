# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · **🇫🇷 Français** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Client conducteur Android natif. Il **ne remplace pas Driver Browser** et ne crée pas de protocole serveur séparé : il utilise la même API Control et les mêmes règles de trajet.

Portée actuelle : Kotlin + Jetpack Compose ; Android 16/API 36, minSdk 26 ; Driver URL ; loopback Emulator via `10.0.2.2` ; snapshot du trajet ; complete/reorder ; géométrie routière optionnelle ; position GPS ponctuelle en premier plan vers Control ; téléchargement Control→Android avec sauvegarde native ; envoi fichier/photo/vidéo Android→Control via sélecteurs système ; restauration de la dernière Driver URL/du mode Emulator ; localisations uk/en/de/fr/es/ko/ja/zh-CN ; tests protocole/unitaires et CI Emulator install/launch/screenshot.

HTTP non chiffré uniquement en debug local ; le chemin public est HTTPS-first.

À venir : carte native, capture caméra, GPS en arrière-plan, smoke Emulator connecté à un vrai trajet synthétique Control, file offline/retry, durcissement Keystore, push et signature production.
