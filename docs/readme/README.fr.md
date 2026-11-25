# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · **🇫🇷 Français** · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Statut :** développement actif. La préversion de test actuelle est **v0.1.0-preview.5**. Il n’existe pas encore de produit stable et finalisé.

## À propos

**cargo-dispatch** est un système de dispatch pour une flotte de petits camions (poids total jusqu’à 7,5 t). Il doit sélectionner des chargements, construire et recalculer des tournées, les transmettre à l’exécution et permettre au dispatcheur de suivre l’exécution réelle.

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

## Version de test

Préversion actuelle : [v0.1.0-preview.5](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.5).

Le ZIP Windows contient Control, un trajet actif synthétique et un PDF de test pour la validation locale en loopback.

Il s’agit d’une **version de test/préversion**, pas d’une version stable. Preview.5 fige l’état actuel de Control/Driver Browser, la chaîne Valhalla schema 1.2 road geometry et le code source des deux Driver Apps natifs jusqu’à r5 : cartes, caméra/fichiers, file de réessai durable et suivi explicite en arrière-plan. L’asset binaire publié reste pour l’instant le preview Windows Control ; la signature mobile de production/TestFlight reste à faire.

## Étapes restantes

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
