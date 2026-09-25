---
id: ADR-aca6aaeb3a5f
type: adr
slug: le-plugin-est-un-seul-binaire-rust-herdr-ank-don
title: Le plugin est un seul binaire Rust, herdr-ank, dont chaque entrée du manifeste est une sous-commande
created: 2026-09-25T16:51:34Z
author: haksolot@omarchy
status: proposed
scope:
  - herdr-plugin.toml
  - Cargo.toml
  - src/**
constraint: |
  Toute commande déclarée dans herdr-plugin.toml (build, startup, actions, panes, events, link_handlers) invoque le binaire herdr-ank produit par cargo build --release, ou ank tui à travers lui. Aucun script shell, Python ou Node n'est une entrée du manifeste. Le plugin cible linux et macos ; windows n'est pas déclaré tant que le socket nommé n'est pas couvert.
schema: 4
version: 1
---

Le périmètre v1 choisi le 2026-09-25 est large : token de tâche par agent dans la
sidebar, action « travailler une tâche » qui crée un worktree et démarre un agent,
pane `ank tui`, notifications. Trois de ces quatre fonctions manipulent du JSON
dans les deux sens (documents `--json` d'ank sous contrat 1, NDJSON du socket
herdr, `HERDR_PLUGIN_CONTEXT_JSON`). Un shell n'y survit pas, et un runtime
interprété ajoute une exigence que ni ank ni herdr ne posent.

Rust est le langage d'ank et de herdr. Un seul binaire `herdr-ank` avec une
sous-commande par entrée du manifeste (`daemon`, `sync`, `work`, `tui`) garde
le manifeste lisible : chaque ligne `command = ["target/release/herdr-ank", "…"]`
se relit sans ouvrir un fichier. Le prix est `[[build]] cargo build --release`
à l'installation, donc cargo chez l'utilisateur et quelques minutes. `herdr
plugin link` saute le build, ce qui convient au développement.

Windows est exclu du manifeste plutôt que promis : herdr y parle par named pipe
et rien ici n'est testé dessus.
