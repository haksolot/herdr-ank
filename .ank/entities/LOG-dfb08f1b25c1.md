---
id: LOG-dfb08f1b25c1
type: log
title: "Mesure session herdr réelle, binaire release de task/51da : env -i HOME=$HOME"
created: 2026-09-26T09:14:12Z
author: haksolot@vmi3223161/ank-4fbf
scope:
  - src/ank/mod.rs
  - src/daemon/mod.rs
  - src/main.rs
  - tests/sync.rs
  - tests/ank_client.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - tests/daemon.rs
about: TASK-51da8857317b
seq: 2
schema: 4
version: 1
---

 PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin HERDR_BIN_PATH=/home/haksolot/.local/bin/herdr … herdr-ank sync -> '3 report(s)', exit 0 (avant le correctif, même commande : 'cannot run ank: No such file or directory' x4 et '0 report(s)'). pane list ensuite : w3A:p2 display_agent 'claude · TASK-51da', tokens ank_task=TASK-51da, ank_expires=59, ank_queue=1 ; w3A:p1 et p3 'claude · ank'. ank trouvé par ~/.local/bin (dossier de HERDR_BIN_PATH aussi, herdr y vit). Deux tests d'ank_client figeaient le nom nu 'ank' : réécrits pour vérifier que client et ank tui lancent le même ank::program().
