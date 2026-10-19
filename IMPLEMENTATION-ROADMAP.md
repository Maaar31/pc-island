# Feuille de route d'implémentation Island

Ce fichier suit les améliorations retenues avant le build final. Chaque élément
doit être implémenté, vérifié, puis marqué comme terminé avec un commit dédié
ou regroupé avec un autre changement étroitement lié.

## Déjà implémenté

- [x] Fenêtre native Tauri 2, transparente, always-on-top, sans focus ni barre des tâches.
- [x] Activation au survol du bord supérieur avec délais 120/250 ms.
- [x] Click-through au repos et région native synchronisée avec l'animation.
- [x] Support du multi-écran et du DPI.
- [x] Interface en niveaux principal/étendu avec navigation par pages.
- [x] Mascotte CSS originale et animations inspirées des principes de Coucou.
- [x] Média Windows : titre, artiste, lecture/pause, précédent, suivant.
- [x] Volume système réel et mute/unmute.
- [x] Wi-Fi et Bluetooth avec gestion de l'indisponibilité.
- [x] Pomodoro côté Rust.
- [x] Presse-papier texte courant.
- [x] Historique local des 20 derniers textes et recherche.
- [x] Masquage devant une fenêtre plein écran.
- [x] Icône tray avec sortie et autostart Windows.
- [x] Packaging NSIS de test.
- [x] Polls média/volume/Pomodoro suspendus lorsque l'île est fermée.

## À implémenter dans l'ordre

1. [x] Page Réglages persistante : animations, plein écran, historique clipboard,
   durée Pomodoro et autostart.
2. [x] Notification locale à la fin du Pomodoro, avec suivi maintenu pendant
   la fermeture de l'île lorsqu'un minuteur est lancé.
3. [~] Luminosité : contrôle reporté, car aucune API fiable et générique ne
   permet de piloter les écrans externes ; aucun faux slider n'est affiché.
4. [x] États média enrichis : progression et durée lorsque le lecteur les expose.
5. [~] Gestion des erreurs et périphériques : les appels échoués affichent un
   état explicite ou `Indisponible`; la détection automatique de changement de
   périphérique reste à approfondir.
6. [x] Presse-papier : suppression d'un élément, copie directe depuis l'historique,
   persistance locale facultative.
7. [x] Animations et shell : mode animations réduites et transitions vivantes
   inspirées de Coucou sans reprendre ses assets.
8. [ ] Tests manuels finaux : deux écrans, DPI mixte, lecteur média, audio,
   radios, plein écran, autostart et installation.
9. [ ] Build release NSIS et vérification de l'installateur final.

## Reporté ou volontairement exclu

- [ ] Historique complet des notifications Windows : accès non fiable pour une
  application Tauri non packagée.
- [ ] Historique système Win+V : API non accessible ; Island conserve son propre
  historique texte limité.
- [ ] Ne pas déranger : absence d'API Windows simple et stable pour ce périmètre.
- [ ] Réseau, compte, télémétrie, IA et synchronisation cloud : hors cahier des
  charges et interdits par conception.
