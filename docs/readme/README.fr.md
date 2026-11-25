# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · **🇫🇷 Français** · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

> **Statut :** développement actif. La préversion de test actuelle est **v0.1.0-preview.3**. Il n’existe pas encore de produit stable et finalisé.

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

## Driver Browser

Driver Browser est **un seul produit**, et non des versions téléphone et desktop séparées. Il possède trois dispositions normales :

- téléphone étroit ;
- tablette ;
- desktop/ordinateur portable large.

Les données, les actions de trajet, les fichiers et le backend sont communs ; seule la disposition change selon la taille de l’écran.

## Carte

Control et Driver Browser disposent maintenant d’une vraie carte interactive avec les points du trajet et la dernière position acceptée du conducteur.

Il ne s’agit **pas encore de la géométrie routière complète**. L’intégration de la géométrie réelle via Valhalla reste à faire.

## Version de test

Préversion actuelle : [v0.1.0-preview.3](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.1.0-preview.3).

Le ZIP Windows contient Control, un trajet actif synthétique et un PDF de test pour la validation locale en loopback.

Il s’agit d’une **version de test/préversion**, pas d’une version stable.

## Étapes restantes

- validation manuelle locale dans le navigateur ;
- test du même Driver Browser sur un vrai téléphone dans le même LAN ;
- chemin HTTPS/adressage public ;
- validation GPS complète du navigateur via HTTPS ;
- tests terrain sur différents réseaux/NAT/CGNAT ;
- géométrie routière réelle Valhalla ;
- futur Driver App natif ;
- version stable.

## Langues de l’interface

L’interface actuelle implémente et vérifie **l’ukrainien, l’anglais et l’allemand**. Les autres traductions du README sont des langues de documentation ou des langues cibles et ne signifient pas encore que l’interface les prend en charge.

## Documentation

Points d’entrée publics : [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md) et [PROJECT_STATE.md](../../PROJECT_STATE.md).

La documentation de travail détaillée du projet est conservée en privé.

## Licence

Logiciel propriétaire © 2026 Roman Zavada. La visibilité publique du dépôt n’accorde aucun droit de licence ouverte. Voir [LICENSE.md](../../LICENSE.md) et [COPYRIGHT.md](../../COPYRIGHT.md).
