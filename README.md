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

## Prérequis de validation

Rust MSVC, les Build Tools Visual Studio, Node 20+ et WebView2 sont requis.
Sur la machine auditée, Node et WebView2 sont présents mais Rust et MSVC
manquent encore.
