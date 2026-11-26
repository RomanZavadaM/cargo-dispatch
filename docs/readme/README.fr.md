# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · **🇫🇷 Français** · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Statut :** développement actif. La version stable actuelle est **v0.1.0**. Le prerelease de test actuel est **v0.1.0-preview.8**, avec désormais un Planner séparé pour Windows, macOS et Linux. La localisation complète en 8 langues est intégrée.

## À propos

**cargo-dispatch** est un système de dispatch pour les flottes de fret. Il doit sélectionner des chargements, construire et recalculer des tournées, les transmettre à l’exécution et permettre au dispatcheur de suivre l’exécution réelle.

Le principal langage de développement est **Rust**.

## Fonctionnalités déjà présentes

- application desktop **Control** séparée basée sur Tauri/Leptos ; Control peut importer un ZIP Planner ou créer manuellement un nouveau trajet actif sans ZIP Planner en utilisant le même état durable du trajet ;
- application desktop **Planner** séparée basée sur Tauri/Leptos : brouillon d’itinéraire manuel, ajout/suppression/réorganisation des points, coordonnées de navigation et révision gérée par le cœur Planner ; le brouillon est stocké dans un journal append-only versionné dans les données de l’application et restauré après redémarrage ;
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

Control et Driver Browser disposent déjà d’une vraie carte de base interactive avec les points du trajet et la dernière position acceptée du conducteur.

Le `main` actuel utilise le schema 1.4 avec une base de planification immuable ; les anciens schemas restent lisibles. Un profil de routage Planner sert uniquement à un calcul de planification précis et ne crée pas de liaison permanente entre le trajet et un véhicule. Après l’import, Control maintient séparément le contexte de routage réel et modifiable de la session active.

Control conserve la géométrie dérivée et recalcule la ligne routière via le service Valhalla runtime configuré après un changement d’ordre. Une réponse de routage tardive ne peut pas écraser un état de trajet plus récent. Si Valhalla n’est pas configuré ou est temporairement indisponible, le nouvel ordre des points est tout de même enregistré et la géométrie dérivée peut être absente.

Planner desktop possède maintenant son propre aperçu Valhalla runtime : l’opérateur fournit l’adresse Valhalla, l’ID du jeu de cartes et les délais, vérifie `/status`, et `/route` est calculé uniquement avec un profil de planification explicitement fourni. Les contraintes numériques inconnues ne sont pas inventées. Un résultat réussi trace la vraie polyline routière sur OSM ; aucune ligne droite fictive n’est dessinée sans résultat. L’aperçu est dérivé de la route/révision/du profil : modifier le brouillon ou le profil de planification efface l’ancienne ligne, et une réponse tardive est rejetée. Un échec de routage ne modifie pas le brouillon Planner.

Planner desktop exporte désormais aussi le package de trajet versionné existant en schema 1.4 via une boîte native « Enregistrer sous ». Le package contient une planning basis explicite (type/classe de véhicule et cargaisons), l’ordre des points et, lorsqu’il existe, uniquement le snapshot correspondant du planning routing profile et de la géométrie Valhalla. Le TripId reste stable pour un brouillon, la TripVersion continue après redémarrage, le CargoId ne dépend pas de la position de la ligne et un ZIP existant n’est jamais écrasé.

## Test v0.1.0-preview.8

[Ouvrir Test v0.1.0-preview.8](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.8).

Il s’agit d’un **prerelease**, pas de la version Stable. C’est une version de test intégrée du produit actuel, construite depuis une source exacte unique.

Le release contient :
- Windows Control x64 ZIP ;
- macOS Control universal ZIP — Intel + Apple Silicon ;
- Linux Control x64 tar.gz ;
- Windows Planner x64 ZIP ;
- macOS Planner universal ZIP — Intel + Apple Silicon ;
- Linux Planner x64 tar.gz ;
- Android Driver debug APK ;
- iOS/iPadOS Driver Simulator ZIP ;
- Linux Addressing Gateway x64 package ;
- SHA256SUMS.

preview.8 inclut déjà le brouillon Planner durable, l’aperçu routier réel Valhalla, l’export ZIP versionné schema 1.4 avec planning basis et la création manuelle d’un trajet actif dans Control sans ZIP Planner. Le profil Planner n’est pas une liaison permanente trajet→véhicule ; le contexte véhicule/appareil réel est géré séparément par Control.

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

- validation manuelle locale dans le navigateur ;
- test du même Driver Browser sur un vrai téléphone dans le même LAN ;
- chemin HTTPS/adressage public ;
- validation GPS complète du navigateur via HTTPS ;
- tests terrain sur différents réseaux/NAT/CGNAT ;


## Langues de l’interface

Control, Planner, Driver Browser, Android Driver et iOS/iPadOS Driver sont implémentés et vérifiés avec le même **ensemble de 8 langues : uk, en, de, fr, es, ko, ja, zh-Hans/zh-CN**. Control et Driver Browser utilisent la langue système avec un choix manuel mémorisé, et le CI vérifie la couverture complète des 8 localisations.

## Documentation

Points d’entrée publics : [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) et [PROJECT_STATE.md](../../PROJECT_STATE.md).

La documentation de travail détaillée du projet est conservée en privé.

## Licence

Logiciel propriétaire © 2026 Roman Zavada. La visibilité publique du dépôt n’accorde aucun droit de licence ouverte. Voir [LICENSE.md](../../LICENSE.md) et [COPYRIGHT.md](../../COPYRIGHT.md).
