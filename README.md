# Island

Projet Tauri 2 neuf pour Windows, construit à partir de l'audit de la
mécanique de fenêtre de Coucou.

## Phase 2

Cette phase implémente uniquement la mécanique de fenêtre avec un panneau
arrondi vide :

- fenêtre Island native fixe de `520 x 340` ;
- fenêtre de réveil transparente de `240 x 10` ;
- ouverture au survol après `120 ms` ;
- fermeture `250 ms` après la sortie ;
- animations CSS de `280 ms` ;
- absence de décoration, barre des tâches et focus ;
- click-through au repos et région native limitée au panneau ouvert ;
- correction de zoom WebView2 pour un `devicePixelRatio` différent ;
- configuration centralisée dans `src/config.ts`.

Le front est du TypeScript sans framework : il n'y a pas encore de contenu
applicatif, et cette option réduit les dépendances pour valider la mécanique.

## Phase 3

La structure de l'interface est maintenant en place avec des données factices :

- niveau principal : média, volume et luminosité ;
- niveau étendu : onglets Notifications, Pomodoro et Presse-papier ;
- composants visuels de base pour boutons, contrôles média, sliders, onglets et
  cartes ;
- retour au niveau principal via le bouton de réduction ou à la fermeture de
  l'île.

Les modules ne contactent encore aucune API Windows et restent explicitement
indiqués comme en préparation.

## Tester l'interface sans Rust

Avec Node installé, lancer `npm run dev`, puis ouvrir `http://localhost:1420`.
En mode navigateur, la zone de test est matérialisée par un petit repère au
milieu du bord supérieur : la survoler ouvre l'île après 120 ms et la quitter
la referme après 250 ms. Le lancement Tauri natif nécessite en plus Rust MSVC
et les Build Tools Visual Studio.

## Prérequis de validation

Rust MSVC, les Build Tools Visual Studio, Node 20+ et WebView2 sont requis.
Sur cette machine, Rust `1.99.1`, Node `22.18.0`, WebView2 `155.0.4283.62`
et les Build Tools MSVC 2019 sont désormais disponibles.

## Phase 4 — modules système

- Média Windows : lecture/pause, précédent, suivant et état du lecteur.
- Volume de sortie Windows via Core Audio.
- Wi-Fi et Bluetooth via `Windows.Devices.Radios`, avec état `Indisponible`
  si Windows ou les droits ne permettent pas l'accès.
- Luminosité volontairement non affichée tant qu'un backend fiable pour les
  écrans compatibles n'est pas disponible.
- Presse-papier texte explicite : le bouton `Lire` récupère le contenu
  actuellement présent dans Windows, et `Copier vers Windows` remplace ce
  contenu. Island ne prétend pas accéder à l'historique Win+V.

## Phase 5 — finitions livrées

- Les rafraîchissements média, volume et Pomodoro sont démarrés à l'ouverture
  de l'île et arrêtés à sa fermeture.
- L'île ne se déclenche pas lorsqu'une fenêtre au premier plan occupe
  entièrement le moniteur, afin de ne pas recouvrir un jeu ou une vidéo plein
  écran.
- Une icône de zone de notification fournit maintenant un menu `Quitter
  Island`, indispensable puisque la fenêtre n'a volontairement pas de bouton
  système.
- Le menu de zone de notification permet d'activer ou désactiver l'autostart
  Windows pour l'utilisateur courant.
- Un historique local de 20 textes est capturé côté Rust et consultable depuis
  la page Presse-papier ; il ne lit toujours pas l'historique système Win+V.
- Le volume possède maintenant un bouton mute/unmute réel via Core Audio.
- L'historique presse-papier dispose d'une recherche locale instantanée.
- Le bundle NSIS est activé en installation pour l'utilisateur courant.
  Test validé avec `npm run tauri build -- --debug`.

## Limites restantes

- La luminosité n'est pas affichée sans backend fiable pour le moniteur.
- Les notifications Windows restent reportées : l'accès à l'historique est
  restreint pour une application Tauri non packagée.
- Le presse-papier actuel est volontairement limité au texte courant ; il ne
  reproduit pas l'historique Win+V et ne conserve pas les images ou fichiers.
- Le mute agit sur la sortie audio système par défaut, pas sur une application
  individuelle.
