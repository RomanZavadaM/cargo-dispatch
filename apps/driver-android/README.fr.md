# Cargo Dispatch Driver — squelette Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · **🇫🇷 Français** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Premier slice Android natif. Il **ne remplace pas Driver Browser** et n’introduit pas un second protocole serveur : il utilise la même API Control.

Portée actuelle : Kotlin + Jetpack Compose, Android 17/API 37, connexion par Driver URL, snapshot du trajet actif, complete/reorder, modèle de géométrie routière, état de reconnexion/erreur, tests de protocole/unitaires et localisations uk/en/de/fr/es/ko/ja/zh-CN.

Dans Android Emulator, `127.0.0.1`/localhost de l’hôte est mappé vers `10.0.2.2`. Le HTTP non chiffré n’est permis qu’en debug local ; le chemin public doit utiliser HTTPS.

Pas encore implémenté : caméra/photo/vidéo, ouverture/partage de documents, GPS en arrière-plan, file offline, session persistante sécurisée, push, signature de production.
