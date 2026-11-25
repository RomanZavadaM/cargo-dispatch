# Cargo Dispatch Driver — iOS/iPadOS

[🇺🇦 Українська](README.md) · [🇬🇧 English](README.en.md) · [🇩🇪 Deutsch](README.de.md) · **🇫🇷 Français** · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Client conducteur Apple natif. Une seule cible SwiftUI prend en charge iPhone et iPad et utilise **la même API Control** que Driver Browser et l’application Android.

Portée actuelle : Swift 6 + SwiftUI + URLSession ; iOS/iPadOS 17+ ; Driver URL ; snapshot du trajet ; complete/reorder ; identifiants permanents + révision locale ; géométrie routière optionnelle ; position CoreLocation ponctuelle en premier plan vers Control ; téléchargement Control→iPhone/iPad via fileExporter ; fichier/photo/vidéo iPhone/iPad→Control via fileImporter + PhotosPicker ; restauration de la dernière Driver URL ; texte de permission localisation traduit ; uk/en/de/fr/es/ko/ja/zh-Hans ; tests de protocole Swift et CI Simulator build/install/launch/screenshot.

HTTP local uniquement en Debug Simulator/LAN ; Release reste HTTPS-first.

À venir : carte MapKit native, capture caméra, GPS arrière-plan, smoke Simulator connecté à un vrai trajet synthétique Control, file offline/retry, durcissement Keychain, push et signature/TestFlight.
