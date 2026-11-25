# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · **🇫🇷 Français** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Client conducteur Android natif. Il **ne remplace pas Driver Browser** et ne crée pas de protocole serveur séparé : il utilise la même API Control et les mêmes règles de trajet.

Portée actuelle : Kotlin + Jetpack Compose ; Android 16/API 36, minSdk 26 ; Driver URL ; snapshot du trajet ; complete/reorder ; carte native MapLibre avec géométrie routière, points et dernière position ; localisation ponctuelle au premier plan ; transfert fichier/photo/vidéo et caméra système ; file durable de réessai pour la dernière position/les médias ; suivi d’arrière-plan Start/Stop explicite via un service de localisation au premier plan avec politique 15 s / 25 m ; récupération de session sécurisée, le token étant chiffré AES/GCM par Android Keystore tandis que seuls la base URL et le mode Emulator non secrets restent dans les préférences privées ; migration automatique de l’ancienne Driver URL brute après reconnexion réussie ; localisations uk/en/de/fr/es/ko/ja/zh-CN ; tests protocole/unitaires et smoke Emulator connecté à un vrai trajet synthétique Control.

HTTP non chiffré uniquement en debug local ; le chemin public est HTTPS-first.

À venir : validation LAN/suivi d’arrière-plan sur appareil réel, parcours HTTPS/public/NAT/CGNAT, push/notifications et signature production.
