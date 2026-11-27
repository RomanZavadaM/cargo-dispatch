# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · **🇫🇷 Français** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Client conducteur Apple natif. Une seule cible SwiftUI prend en charge iPhone et iPad et utilise **la même API Control** que Driver Browser et l’application Android.

Portée actuelle : Swift 6 + SwiftUI + URLSession ; iOS/iPadOS 17+ ; Driver URL ; snapshot du trajet ; complete/reorder ; carte MapKit native avec géométrie routière, points et dernière position ; localisation CoreLocation ponctuelle au premier plan ; transfert fichier/photo/vidéo et caméra système ; file durable de réessai pour les positions et les médias ; suivi CoreLocation d’arrière-plan Start/Stop explicite avec `UIBackgroundModes=location` et filtre 25 m ; récupération de session sécurisée avec le token dans un élément Keychain non synchronisé (`AfterFirstUnlockThisDeviceOnly`) et seulement la base URL non secrète dans UserDefaults ; migration automatique de l’ancienne Driver URL brute après reconnexion réussie ; uk/en/de/fr/es/ko/ja/zh-Hans ; tests de protocole Swift et smoke Simulator connecté à un vrai trajet synthétique Control.

HTTP local uniquement en Debug Simulator/LAN ; Release reste HTTPS-first.

**Au quotidien pour le conducteur**

- bouton « Ouvrir dans le navigateur GPS » sur chaque point non effectué ;
- « Marquer comme fait » demande une confirmation ;
- les nouveaux fichiers peuvent être rattachés à un point ou à tout le trajet ;
- l'écran ouvert s'actualise toutes les 15 secondes et indique s'il y a une liaison ;
- sans liaison, le parcours est conservé (jusqu'aux 500 dernières positions) puis envoyé dans l'ordre ;
- un fichier refusé par le programme de régulation reste visible : le renvoyer ou le retirer ;
- messages d'erreur dans la langue du téléphone, sans termes internes.

À venir : validation LAN/suivi d’arrière-plan sur appareil réel, parcours HTTPS/public/NAT/CGNAT, push/notifications et signature production/TestFlight.
