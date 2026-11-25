# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · **🇫🇷 Français** · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Statut :** développement actif. La version stable actuelle est **v0.1.0**. Après la localisation complète, le prochain grand axe sera le Planner d’itinéraire.

## À propos

**cargo-dispatch** est un système de dispatch pour les flottes de fret. Il doit sélectionner des chargements, construire et recalculer des tournées, les transmettre à l’exécution et permettre au dispatcheur de suivre l’exécution réelle.

Le principal langage de développement est **Rust**.

## Fonctionnalités déjà présentes

- application desktop **Control** séparée basée sur Tauri/Leptos ;
- stockage local durable du trajet actif et restauration après redémarrage ;
- paquet de trajet versionné entre Planner et Control ;
- un seul **Driver Browser** adaptatif pour téléphone, tablette, ordinateur portable et navigateur desktop ;
- modification de l’ordre des points ouverts dans Control et Driver Browser ;
- PDF et fichiers **Control → Driver Browser** ;
- fichiers, photos et vidéos **Driver Browser → Control** avec sauvegarde locale ;
- deux clients navigateur simultanés pour un même trajet et reconnexion sans base navigateur séparée ;
- carte interactive OpenStreetMap dans Control et Driver Browser ;
- modes loopback et LAN explicite pour les tests et l’utilisation locale.

## Applications Driver natives

Les deux clients natifs sont déjà intégrés dans `main` :

- **Android** — Jetpack Compose, route/validation/réorganisation, fichiers/photos/vidéos, GPS au premier plan, restauration de session, carte native MapLibre + OpenStreetMap, géométrie routière actuelle, points/dernière position et capture photo/vidéo système ; build/install/lancement/capture Emulator PASS.
- **iPhone/iPad** — SwiftUI, route/validation/réorganisation, fichiers/photos/vidéos, CoreLocation au premier plan, restauration de session, MapKit natif, géométrie actuelle, points/dernière position et capture photo/vidéo système ; build/install/lancement/capture Simulator PASS.

Les deux utilisent la même API Control et ne créent pas de base de trajet séparée.

Depuis r4/r5, les deux clients natifs disposent aussi d’une file durable de réessai pour la position et les médias. Android r5 ajoute un suivi d’arrière-plan Start/Stop explicite via un service de localisation au premier plan, une notification Stop persistante et une politique 15 s / 25 m. Apple r5 ajoute un suivi CoreLocation d’arrière-plan Start/Stop explicite, `UIBackgroundModes=location`, un filtre de 25 m et ne demande pas l’autorisation `Always` sans besoin distinct.

## Driver Browser

Driver Browser est **un seul produit**, et non des versions téléphone et desktop séparées. Il possède trois dispositions normales :

- téléphone étroit ;
- tablette ;
- desktop/ordinateur portable large.

Les données, les actions de trajet, les fichiers et le backend sont communs ; seule la disposition change selon la taille de l’écran.

## Carte

Control et Driver Browser disposent maintenant d’une vraie carte interactive avec les points du trajet et la dernière position acceptée du conducteur.

La branche `main` contient maintenant une chaîne de géométrie routière versionnée (schéma 1.2) : Planner peut transmettre une géométrie dérivée, Control la conserve et les deux cartes n’affichent la ligne routière que tant qu’elle correspond à l’ordre actuel des arrêts. Tout changement d’ordre invalide immédiatement l’ancienne ligne.

Il reste à connecter le **transport HTTP réel vers Valhalla et le recalcul automatique de la géométrie**. Le contrat et l’affichage sont présents ; le moteur de routage externe n’est pas encore relié au runtime produit.

## Stable v0.1.0

[Ouvrir Stable v0.1.0](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0).

La version Stable est reconstruite depuis une seule source exacte et publie :
- Windows Control x64 ZIP ;
- macOS Control universal ZIP — Intel + Apple Silicon ;
- Linux Control x64 tar.gz ;
- Android Driver debug APK pour installation de test ;
- iOS/iPadOS Driver Simulator ZIP ;
- package Linux Addressing Gateway x64 avec Docker/Compose ;
- SHA256SUMS.

**Linux Control** est l’application desktop de l’opérateur. **Linux Addressing Gateway** est le module serveur relay séparé ; ce sont deux packages différents.

Les packages Android et Apple ne sont pas encore signés pour les stores. App Store/TestFlight/Google Play et la signature de production seront traités séparément plus tard.

## Étapes restantes

- UI desktop Planner/editor et builds de release pour Windows, macOS et Linux ;

- validation manuelle locale dans le navigateur ;
- test du même Driver Browser sur un vrai téléphone dans le même LAN ;
- chemin HTTPS/adressage public ;
- validation GPS complète du navigateur via HTTPS ;
- tests terrain sur différents réseaux/NAT/CGNAT ;
- transport HTTP Valhalla réel et recalcul de la géométrie après changement d’ordre ;
- stockage sécurisé de la session Driver avec Android Keystore + Apple Keychain et migration de l’ancienne URL brute ;
- version stable.


## Langues de l’interface

L’interface desktop/browser intégrée implémente actuellement **l’ukrainien, l’anglais et l’allemand**. Les nouveaux clients natifs Android et iOS/iPadOS démarrent directement avec les packs **uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**, et leur documentation suit le même ensemble de langues.

## Documentation

Points d’entrée publics : [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) et [PROJECT_STATE.md](../../PROJECT_STATE.md).

La documentation de travail détaillée du projet est conservée en privé.

## Licence

Logiciel propriétaire © 2026 Roman Zavada. La visibilité publique du dépôt n’accorde aucun droit de licence ouverte. Voir [LICENSE.md](../../LICENSE.md) et [COPYRIGHT.md](../../COPYRIGHT.md).
