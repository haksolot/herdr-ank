---
id: LOG-1ea5d4a37cdd
type: log
title: "Daemon : Binary::of_plugin relève taille et mtime de $HERDR_PLUGIN_ROOT/bin/herdr-ank(.exe) en tout"
created: 2026-09-26T09:22:34Z
author: haksolot@vmi3223161/ank-4fbf
scope:
  - src/daemon/mod.rs
  - tests/daemon.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-8375b6128604
seq: 1
schema: 4
version: 1
---

 premier dans run() ; à chaque tour de boucle, remplacé ou absent -> drop du verrou, relance détachée de ce binaire s'il existe (attente bornée à 2 s sur ExecutableFileBusy), sortie 0. Découvertes sous charge : (1) relever l'empreinte après le verrou laissait un binaire disparu entre-temps enregistré comme absent, comparé absent = absent, daemon éternel ; corrigé en relevant avant et en tenant l'absence pour un départ. (2) ETXTBSY au lancement d'une copie fraîche quand un test parallèle fork au même moment ; les tests réessaient sur cette seule erreur. 45 passages de la suite sous charge sans échec, aucun daemon de test survivant.
