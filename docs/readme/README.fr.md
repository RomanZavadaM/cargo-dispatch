# cargo-dispatch

[🇺🇦 Українська](../../README.md) · [🇬🇧 English](README.en.md) · **🇫🇷 Français** · [🇩🇪 Deutsch](README.de.md) · [🇪🇸 Español](README.es.md) · [🇰🇷 한국어](README.ko.md) · [🇯🇵 日本語](README.ja.md) · [🇨🇳 简体中文](README.zh.md)

Régulation d'une flotte de fret : planifier un trajet, le confier à l'exécution, rester en contact avec le conducteur et consigner ce qui s'est réellement passé. Le langage principal de développement est **Rust**.

**Version stable : [v0.2.0](https://github.com/RomanZavadaM/cargo-dispatch/releases/tag/v0.2.0).** Le développement continue ; les versions d'essai figurent dans [Releases](https://github.com/RomanZavadaM/cargo-dispatch/releases).

## Composants

| Programme | Utilisateur | Plateformes |
|---|---|---|
| **Control** | régulateur : suit les trajets en cours | Windows, macOS, Linux |
| **Planner** | planificateur : prépare l'itinéraire et le paquet de trajet | Windows, macOS, Linux |
| **Page conducteur** | conducteur, dans tout navigateur, sans installation | téléphone, tablette, ordinateur |
| **Driver pour Android** | conducteur | Android 8.0 ou plus récent |
| **Driver pour iPhone et iPad** | conducteur | iOS / iPadOS 17 ou plus récent |
| **Addressing Gateway** | liaison par internet | serveur Linux |

Control est la seule source de vérité d'un trajet en cours. Il fournit au conducteur la page, l'itinéraire et les fichiers, et reçoit les validations, les positions et les pièces jointes. Il n'existe pas de base centrale des trajets.

## Premiers pas

1. Téléchargez une archive depuis la page de la version, décompressez-la et lancez **Control**. Le paquet Control contient des données d'essai fictives (`local-browser-test-data`).
2. **« Ajouter un trajet »** → « Ouvrir un trajet existant » (dossier de trajet ou paquet Planner) ou « Créer un nouveau trajet manuellement ».
3. **« Connecter les conducteurs »** → « Cet ordinateur », « Même réseau Wi-Fi » ou « Partout, par internet ».
4. Transmettez au conducteur le lien du bloc **« Que faire ensuite »** : « Ouvrir » ou « Copier ».
5. Le conducteur ouvre le lien dans un navigateur ou le colle dans l'application Driver.

## Fonctions

**Control**
- plusieurs trajets à la fois, une fenêtre par trajet ;
- trajet issu d'un paquet Planner ou créé à la main ; l'état est enregistré localement et rétabli après un redémarrage ;
- ordre des points ouverts, carte, dernière position du conducteur, comparaison prévu / réalisé ;
- fichiers vers le conducteur ; fichiers, photos et vidéos reçus du conducteur ;
- trouve lui-même l'adresse de l'ordinateur sur le réseau local ; le mode de liaison change sans arrêt manuel;
- messages d'erreur dans la langue de l'interface ; heure de position, taille de fichier et nom du point en clair.

**Planner**
- brouillon d'itinéraire : ajouter, retirer, réordonner des points par coordonnées ; le brouillon survit à un redémarrage ;
- aperçu de l'itinéraire et tracé routier via votre serveur Valhalla avec des paramètres de véhicule saisis explicitement (les valeurs inconnues ne sont pas inventées) ;
- export d'un paquet de trajet (ZIP, schéma 1.4) avec les marchandises et la base de planification ; un fichier existant n'est jamais écrasé.

**Conducteur : navigateur, Android, iPhone et iPad**
- itinéraire et carte, point marqué comme fait, réordonnancement des points ouverts ;
- réception de documents ; envoi de fichiers, photos et vidéos ;
- partage de la position ; dans les applications aussi en arrière-plan, activé et arrêté explicitement, avec file de renvoi;
- la validation d'un point demande une confirmation ; les applications offrent aussi un bouton de navigation, le rattachement d'un fichier à un point, un écran qui s'actualise seul, la conservation du parcours sans réseau (jusqu'à 500 positions) et les fichiers refusés visibles.

## Liaison et sécurité

- Les modes **Cet ordinateur** et **réseau local** ne sont pas chiffrés (HTTP) : à utiliser seulement sur un réseau de confiance. L'accès réseau ne s'active jamais seul et aucun port du routeur n'est ouvert.
- Un lien avec un code secret donne accès au trajet : ne le transmettez qu'au conducteur de ce trajet.
- Les navigateurs de téléphone n'autorisent généralement la géolocalisation qu'en HTTPS, donc via le Gateway.
- Pour qu'un téléphone atteigne Control sur le réseau, autorisez le programme dans le pare-feu de l'ordinateur.
- **Par internet :** votre propre Gateway derrière un proxy inverse HTTPS. Control s'y connecte lui-même, aucun port entrant n'est nécessaire sur l'ordinateur. Le Gateway ne conserve ni trajets, ni documents, ni médias, ni positions. Limite par défaut : 64 Mio par requête (réglable jusqu'à 512 Mio).

## Cartes et itinéraires

La carte de Control, Planner, du navigateur et d'Android est [OpenStreetMap](https://www.openstreetmap.org/copyright) (© OpenStreetMap contributors) ; sur iPhone et iPad, c'est Apple MapKit. Le fournisseur de cartes voit l'adresse IP de l'appareil et la zone affichée. Le tracé routier nécessite votre propre serveur Valhalla ; sans lui le trajet fonctionne, seul le tracé manque.

## Langues

Tous les programmes existent en українська, English, Français, Deutsch, Español, 한국어, 日本語 et 简体中文. La CI vérifie que les traductions sont complètes ; les détails techniques du système dans les messages d'erreur restent en anglais.

## Pas encore vérifié, et limites

- Les applications Android et iPhone/iPad n'ont pas été essayées sur des appareils physiques — seulement sur émulateur, simulateur et tests automatiques.
- L'internet mobile et le Gateway sur un réseau réel ne sont vérifiés qu'automatiquement.
- Les paquets ne sont pas signés ; Android est un APK de débogage, iOS uniquement une version pour le simulateur Apple.
- Après un redémarrage de Control, trajets et liens reviennent seuls ; la liaison s'active d'un clic (après v0.2.0).
- Le régulateur ne peut pas encore modifier les points ni les marquer comme faits.

## Compilation

Les compilations, tests et versions s'exécutent dans GitHub Actions ; voir `.github/workflows`.

## Documentation

État du projet (en ukrainien) : [START_HERE.md](../../START_HERE.md), [HANDOFF_CURRENT.md](../../HANDOFF_CURRENT.md), [PROJECT_STATE.md](../../PROJECT_STATE.md). La documentation de travail détaillée est tenue en privé.

## Licence

Logiciel propriétaire © 2026 Roman Zavada (Роман Завада). Un dépôt public n'accorde aucun droit de licence libre. Voir [LICENSE.md](../../LICENSE.md), [COPYRIGHT.md](../../COPYRIGHT.md) et [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md).
