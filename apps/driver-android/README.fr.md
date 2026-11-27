# Cargo Dispatch Driver — Android

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · **🇫🇷 Français** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Client conducteur Android natif. Il **ne remplace pas Driver Browser** et ne crée pas de protocole serveur séparé : il utilise la même API Control et les mêmes règles de trajet.

Portée actuelle : Kotlin + Jetpack Compose ; Android 16/API 36, minSdk 26 ; Driver URL ; snapshot du trajet ; complete/reorder ; carte native MapLibre avec géométrie routière, points et dernière position ; localisation ponctuelle au premier plan ; transfert fichier/photo/vidéo et caméra système ; file durable de réessai pour les positions et les médias ; suivi d’arrière-plan Start/Stop explicite via un service de localisation au premier plan avec politique 15 s / 25 m ; récupération de session sécurisée, le token étant chiffré AES/GCM par Android Keystore tandis que seuls la base URL et le mode Emulator non secrets restent dans les préférences privées ; migration automatique de l’ancienne Driver URL brute après reconnexion réussie ; localisations uk/en/de/fr/es/ko/ja/zh-CN ; tests protocole/unitaires et smoke Emulator connecté à un vrai trajet synthétique Control.

HTTP non chiffré uniquement en debug local ; le chemin public est HTTPS-first.

**Au quotidien pour le conducteur**

- bouton « Ouvrir dans le navigateur GPS » sur chaque point non effectué ;
- « Marquer comme fait » demande une confirmation ;
- les nouveaux fichiers peuvent être rattachés à un point ou à tout le trajet ;
- l'écran ouvert s'actualise toutes les 15 secondes et indique s'il y a une liaison ;
- sans liaison, le parcours est conservé (jusqu'aux 500 dernières positions) puis envoyé dans l'ordre ;
- un fichier refusé par le programme de régulation reste visible : le renvoyer ou le retirer ;
- messages d'erreur dans la langue du téléphone, sans termes internes.

À venir : validation LAN/suivi d’arrière-plan sur appareil réel, parcours HTTPS/public/NAT/CGNAT, push/notifications et signature production.
