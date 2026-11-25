# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · **🇫🇷 Français** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Client conducteur Apple natif. Une seule cible SwiftUI prend en charge iPhone et iPad et utilise **la même API Control** que Driver Browser et l’application Android.

Portée actuelle : Swift 6 + SwiftUI + URLSession ; iOS/iPadOS 17+ ; Driver URL ; snapshot du trajet ; complete/reorder ; carte MapKit native avec géométrie routière, points et dernière position ; localisation CoreLocation ponctuelle au premier plan ; transfert fichier/photo/vidéo et caméra système ; file durable de réessai pour la dernière position/les médias ; suivi CoreLocation d’arrière-plan Start/Stop explicite avec `UIBackgroundModes=location` et filtre 25 m ; récupération de session sécurisée avec le token dans un élément Keychain non synchronisé (`AfterFirstUnlockThisDeviceOnly`) et seulement la base URL non secrète dans UserDefaults ; migration automatique de l’ancienne Driver URL brute après reconnexion réussie ; uk/en/de/fr/es/ko/ja/zh-Hans ; tests de protocole Swift et smoke Simulator connecté à un vrai trajet synthétique Control.

HTTP local uniquement en Debug Simulator/LAN ; Release reste HTTPS-first.

À venir : validation LAN/suivi d’arrière-plan sur appareil réel, parcours HTTPS/public/NAT/CGNAT, push/notifications et signature production/TestFlight.
