# Audit de la mécanique de fenêtre Windows de Coucou

Audit réalisé en lecture seule sur `coucou-ref\windows`. Aucun fichier de
`coucou-ref` n'a été modifié.

## Phase 0 — prérequis et compilation

| Prérequis | Résultat |
|---|---|
| Node.js | Présent, `v22.18.0` |
| npm | Présent, `10.9.3` |
| WebView2 Runtime | Présent, `155.0.4283.62` |
| rustup / rustc / cargo | Manquants |
| Outils MSVC (`cl`) | Manquants dans le PATH |
| `npm install` dans `coucou-ref\windows` | Réussi, 19 paquets audités, 0 vulnérabilité npm signalée |
| `npm run pack` | Bloqué avant compilation : `cargo metadata` introuvable |

Le dépôt Windows est bien un projet Tauri 2 : `windows\package.json:1-29`
déclare `@tauri-apps/cli` en version 2 et le script réel `pack` exécute
`tauri build && node scripts/pack.mjs`. Sa configuration est dans
`windows\src-tauri\tauri.conf.json:1-55`.

La compilation complète n'a donc pas pu être vérifiée. Il faut installer
Rust via rustup avec la toolchain MSVC et les Build Tools Visual Studio
correspondants, puis relancer `npm run pack`. WebView2 est déjà installé.

## 1. Création et configuration de la fenêtre

La fenêtre principale est créée par Tauri à partir de la déclaration
`app.windows` dans `windows\src-tauri\tauri.conf.json:12-29`. Elle porte le
label `island`, charge `index.html`, et est configurée avec une taille initiale
de `240 x 6` pixels, sans décoration, transparente, always-on-top, sans barre
des tâches, sans focus initial et visible.

Le code Rust récupère cette fenêtre par son label dans
`windows\src-tauri\src\island.rs:93-95`. La géométrie est ensuite recalculée
dans `apply_geometry` (`island.rs:354-395`) :

- panneau logique maximal : `PANEL_W = 720` et `PANEL_H = 320`
  (`island.rs:17-18`) ;
- bandeau de réveil : `STRIP_W = 240` et `STRIP_H = 6`
  (`island.rs:19-20`) ;
- conversion logique → physique avec `scale_factor`, centrage horizontal et
  position sur le bord supérieur (`island.rs:358-376`) ;
- réapplication de la taille après déplacement entre écrans et nouvelle
  tentative si WebView2 n'a pas encore appliqué la taille demandée
  (`island.rs:378-395`).

Au démarrage, `setup` applique les styles non activants, la géométrie et le
polling (`windows\src-tauri\src\lib.rs:885-898`). Le front signale ensuite la
forme réelle du panneau à Rust avec `set_island_rect`
(`lib.rs:172-180`), appelé depuis `windows\src\island\island.ts:680-689`.

## 2. Transparence, always-on-top, décoration, taskbar et focus

Les propriétés de base sont déclarées dans
`windows\src-tauri\tauri.conf.json:17-29` :

- `decorations: false` ;
- `transparent: true` ;
- `alwaysOnTop: true` ;
- `skipTaskbar: true` ;
- `focus: false` ;
- `shadow: false` ;
- `minimizable`, `maximizable` et `closable` à `false`.

Le démarrage force ensuite `platform::make_non_activating`
(`lib.rs:885-887`). Sous Windows, cette fonction ajoute
`WS_EX_NOACTIVATE` et `WS_EX_TOOLWINDOW` à la fenêtre
(`windows\src-tauri\src\platform\windows.rs:327-336`). Le premier empêche
l'activation par les clics ; le second la sort notamment d'Alt-Tab.

Le code prévoit une exception explicite pour un champ texte :
`set_activating` retire temporairement `WS_EX_NOACTIVATE`
(`platform\windows.rs:339-351`), et la commande Tauri `focus_window` appelle
`set_focus` uniquement quand le front le demande
(`windows\src-tauri\src\lib.rs:182-190`). Pour Island, cette exception ne
devra pas être utilisée dans la version initiale, puisque le brief interdit
la prise de focus.

La fenêtre native reste présente et transparente ; au repos, le front passe
en bandeau de 240 x 6 après une temporisation de 420 ms
(`windows\src\island\island.ts:701-721`). Ce bandeau est la zone invisible
qui permet au système de livrer un événement de souris sans maintenir un
polling actif.

## 3. Click-through et zones interactives

La décision de click-through est prise côté Rust, avec la forme du panneau
transmise par le front (`lib.rs:172-180`). La marge de tolérance vaut 14 px
(`island.rs:22-26`).

Sous Windows, `set_input_region` est volontairement vide :
`platform\windows.rs:356-357`. Le click-through repose sur
`WebviewWindow::set_ignore_cursor_events`, piloté par le thread de polling.

Le polling de `island.rs:412-508` :

1. lit la position physique du curseur ;
2. la convertit en coordonnées locales selon `outer_position` et
   `scale_factor` (`island.rs:447-463`) ;
3. teste l'intersection avec le rectangle du panneau et la marge
   (`island.rs:472-478`) ;
4. maintient la fenêtre interactive sur cette zone et la rend transparente
   aux événements ailleurs (`island.rs:497-504`).

Quand la fenêtre est réduite, `refresh_click_through` donne seulement au
strip de réveil une région interactive (`island.rs:517-543`). Ainsi, la zone
restante ne peut pas avaler les clics des applications sous-jacentes.

Un bouton pressé est traité comme un début possible de glisser-déposer :
`left_button_down` force temporairement l'acceptation des événements sur la
fenêtre afin que la cible OLE reste trouvable
(`island.rs:480-496`). Cette logique est spécifique au dépôt de fichiers et
ne doit pas être reprise dans Island tant que le module n'existe pas.

## 4. Détection du survol du bord supérieur

Il n'y a pas de hook global Windows. La détection repose sur un polling Win32
de la position globale du curseur, démarré dans
`island::spawn_cursor_poll` (`island.rs:412-446`). Sous une plateforme avec
curseur global, la période est de 16 ms, soit environ 60 Hz
(`island.rs:432-434`).

Le thread est bloqué sur une `Condvar` quand le polling n'est pas nécessaire
(`island.rs:52-91`). `set_collapsed` active ou désactive ce thread selon l'état
de la fenêtre (`lib.rs:158-170`) ; la fenêtre réduite n'entretient donc pas
de timer ou de polling permanent.

Le front reçoit les événements `cursor` et met à jour la machine d'états
(`island.rs:504`, `island.ts:829-870`). La zone de survol au repos est le
`#wake-strip`, centré et transparent, de 240 x 6 px
(`windows\src\style.css:61-68`), avec un événement `mouseenter`
(`island.ts:727-730`).

La machine d'états actuelle n'est pas conforme aux délais du brief :
`openOnHover` est désactivé par défaut et la fermeture hover est de 0,6 s
(`windows\src\island\fsm.ts:41-48`), sans délai d'entrée de 120 ms. Il faudra
réécrire cette partie pour Island.

## 5. Multi-écrans, écran actif et DPI

Le choix d'écran est centralisé dans `target_monitor`
(`island.rs:222-239`) :

- préférence persistée par origine logique, nom et résolution ;
- option spéciale `cursor` qui sélectionne l'écran sous le curseur ;
- repli sur l'écran principal puis le premier écran disponible.

Les moniteurs sont décrits en coordonnées logiques avec leur facteur
`scale_factor` (`island.rs:108-157`). La position et la taille physiques sont
converties explicitement dans `apply_geometry` (`island.rs:358-376`).

Les changements de branchement, de résolution ou de scaling sont détectés
environ deux fois par seconde pendant que le polling est actif
(`island.rs:432-446`). Un événement `screen-changed` demande au front de
repositionner la fenêtre.

Le code a aussi une correction spécifique à WebView2 lorsque son
`devicePixelRatio` ne correspond pas au scaling de la fenêtre :
`calculate_zoom_correction` (`island.rs:285-303`) et la commande `fit_zoom`
(`lib.rs:198-216`). Les tests unitaires de ce calcul sont dans
`island.rs:305-352`. C'est une bonne base pour le critère de scaling mixte,
mais elle n'a pas pu être validée par compilation sur cette machine.

## 6. Animation d'ouverture et de fermeture

L'animation est côté front. Le panneau est dessiné à l'intérieur d'une
fenêtre native de taille maximale ; la fenêtre native n'est donc pas redimensionnée
à chaque frame. Le front pousse uniquement le rectangle courant à Rust.

`windows\src\core\anim.ts:1-123` fournit :

- une interpolation cubique `Ease.out` ;
- une courbe de fermeture `cubicBezier(0.45, 0, 0.2, 1)` ;
- `Spring` pour l'ouverture ;
- `Tracked.curveTowards` pour la fermeture sans dépassement.

La page CSS utilise également `will-change` sur largeur, hauteur et
transformation (`windows\src\style.css:75-81`). La logique de changement de
mode se trouve dans `island.ts:311-350`, mais elle est liée aux états Coucou
(`hidden`, `compact`, `expanded`, Mochi, alertes et intégrations). La durée
de fermeture actuelle est de 340 ms dans `anim.ts:104`, et ne correspond pas
exactement aux 280 ms demandées.

## 7. Réutilisation, réécriture et couplages

### Réutilisable comme référence, avec adaptation

- `windows\src-tauri\src\island.rs:1-543` pour les idées de fenêtre fixe,
  polling conditionnel, coordonnées physiques et click-through ;
- `windows\src-tauri\src\platform\windows.rs:327-357` pour les styles
  `WS_EX_NOACTIVATE` / `WS_EX_TOOLWINDOW` et la stratégie Windows ;
- `windows\src-tauri\src\lib.rs:158-216` pour le contrat IPC de géométrie,
  état réduit et correction DPI ;
- `windows\src\core\anim.ts:1-123` pour les primitives d'animation, après
  vérification que leurs constantes conviennent.

Ces fragments sont du code MIT et devront être attribués individuellement si
copiés dans Island.

### À réécrire pour Island

- la configuration Tauri (`tauri.conf.json`) : identité, dimensions, titre,
  bundle, icônes et description ;
- `fsm.ts` et la logique de survol : délai d'entrée 120 ms, fermeture 250 ms,
  aucune ouverture par clic ou raccourci ;
- le front `island.ts` et les styles : rectangle vide uniquement, palette
  propre à Island, niveaux principal/étendu reportés à la phase 3 ;
- le bootstrap Rust et le bridge Tauri, afin de supprimer les commandes
  Coucou non nécessaires ;
- le mécanisme de sortie propre, à prévoir sans reprendre l'identité visuelle
  de Coucou.

### Couplé au reste de Coucou et à exclure

- `src-tauri\src\lib.rs` dans son ensemble : agents, hooks, chat, secrets,
  téléphone, intégrations, tray et réglages ;
- `src-tauri\src\desktop.rs` et `src\island\desktop.ts` : fenêtre Mochi
  secondaire et glisser-déposer du personnage ;
- `src\mochi\`, `src\core\sound.ts`, les sons, les icônes et tous les médias ;
- `src-tauri\src\hooks.rs`, `agents.rs`, `flow\`, `speech\`, `notes\`,
  `phone_link\`, `spotify.rs` et les intégrations réseau ;
- les pages et styles qui portent l'identité Coucou/Mochi.

Le fichier `windows\src-tauri\Cargo.toml:17-88` confirme que la dépendance
Tauri actuelle entraîne de nombreuses dépendances étrangères à la mécanique :
réseau (`reqwest`, TLS, WebSocket), agents et stockage de secrets,
audio/dictée, téléphone et presse-papier. Elles ne doivent pas être
transposées dans Island.

## 8. Dépendances strictement nécessaires à la mécanique

Pour une première mécanique Windows Tauri 2, le minimum estimé est :

- npm : `@tauri-apps/cli`, `@tauri-apps/api`, `typescript`, `vite` ;
- Rust : `tauri` et `tauri-build` ;
- APIs Windows déjà indirectement accessibles via Tauri/tao pour la fenêtre,
  le scaling, le curseur et les styles Win32.

Le crate `windows` explicite du projet Coucou
(`windows\src-tauri\Cargo.toml:53-87`) sert surtout à la lecture média,
audio, processus, hooks et intégrations. Il n'est pas nécessaire pour
commencer la mécanique si Tauri expose les opérations de fenêtre requises.
Il ne faudra l'ajouter qu'en cas de besoin Win32 non couvert par Tauri.

`serde`/`serde_json` peuvent être ajoutés seulement si l'état de configuration
Rust l'exige. `tokio`, `reqwest`, `webview2-com`, `arboard` et les plugins
Coucou ne sont pas requis par le panneau vide.

## 9. Risques

1. **Flicker à l'entrée** : le panneau animé doit conserver le rectangle
   natif maximal et ne publier la nouvelle zone interactive qu'au bon moment ;
   sinon le curseur peut repasser dans une zone click-through pendant
   l'animation.
2. **Perte de focus** : ne jamais retirer `WS_EX_NOACTIVATE` pour le panneau
   vide ; la commande `focus_window` de Coucou est une capacité à exclure.
3. **DPI mixte** : les coordonnées physiques et logiques doivent rester
   séparées ; le changement d'écran doit réappliquer position, taille et
   zoom WebView2.
4. **Applications plein écran** : `alwaysOnTop` peut afficher l'île au-dessus
   d'un jeu ou d'une vidéo plein écran. Le brief demande une option de
   masquage, mais Coucou ne fournit pas ici une solution générale fiable.
5. **Bord supérieur inaccessible** : certains shell Windows, logiciels de
   capture ou configurations multi-écrans peuvent modifier le comportement
   de la souris au bord de l'écran.
6. **Désynchronisation fenêtre/front** : WebView2 peut appliquer la nouvelle
   taille avec retard ; Coucou prévoit une seconde application de taille
   (`island.rs:378-395`), à conserver et à tester.
7. **Glisser-déposer** : le contournement OLE de Coucou est spécifique aux
   dépôts de fichiers et complexifie le click-through ; il doit rester hors
   du MVP.
8. **CPU à l'idle** : le modèle `Condvar` est adapté, mais toute boucle
   ajoutée côté front doit être arrêtée quand l'île est fermée.

## Recommandation

Repartir de zéro dans un projet Tauri 2 neuf, en s'inspirant seulement de la
mécanique de `island.rs`, de `platform\windows.rs` et de `anim.ts`. Ne pas
extraire les fichiers tels quels : ils sont intégrés aux états Coucou, aux
agents, à Mochi, aux intégrations et aux commandes de configuration.

La priorité de la phase 2 sera de reproduire le noyau suivant, avec une
attribution MIT ciblée : fenêtre transparente non activante, bandeau de
réveil 240 x 10, polling uniquement quand nécessaire, click-through par
rectangle et géométrie multi-écrans avec scaling. Les délais et l'animation
devront être implémentés selon le brief plutôt que copiés des valeurs
actuelles de Coucou.

