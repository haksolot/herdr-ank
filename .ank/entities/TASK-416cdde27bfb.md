---
id: TASK-416cdde27bfb
type: task
slug: moteur-de-synchronisation-des-panes-worktrees-et
title: "Moteur de synchronisation : des panes, worktrees et claims aux tokens de la spec"
created: 2026-09-25T16:54:25Z
author: haksolot@omarchy
status: open
scope:
  - src/sync/**
  - tests/sync.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
blocked_by: [TASK-dadb4d2261b5, TASK-de0a51842971]
done_criteria: |
  `sync::plan(panes, agents, corpora, config) -> Vec<Report>` est une fonction pure : elle prend la liste des panes avec cwd, la liste des agents herdr avec nom et pane, et par corpus le résultat de `find --status in_progress` et de `context`, et rend un rapport par pane à mettre à jour, conforme à SPEC-dbe3cf972f71 : tokens `ank_task`, `ank_title`, `ank_expires`, `ank_ambiguous`, `ank_queue`, source `ank:sync`, ttl 90000, et les tokens à effacer. Les tests couvrent : claim attribué par suffixe `/<nom>` (ADR-fa8b6a103597) ; claim à identité de repli sur deux panes du même worktree marqué ambigu ; pane sans agent ou hors corpus qui ne reçoit rien ; claim disparu qui produit un `clear` des cinq tokens.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
schema: 4
version: 3
---

Le cœur du plugin, gardé pur pour être testé sans herdr ni ank : les deux
clients (T2, T3) alimentent l'entrée, `sync::apply` envoie chaque `Report`
par `report_metadata`.

Le mapping cwd → corpus remonte du cwd de la pane au premier `.ank/` ; un
worktree git partage le corpus de son dépôt, `ank status --json` donne
`corpus` (root commit) pour dédupliquer deux chemins d'un même corpus.

`ank_expires` vient de `coordination` de `find --json` (« claimed by X,
expires in 30m ») : lire la forme exacte dans `ank help find --json` et
préférer un champ structuré s'il existe.

EDITS (2)
├── 2026-09-25T16:54:25Z haksolot@omarchy — created (version 0 to 1, produced 04e74e2c19cb)
└── 2026-09-25T17:28:38Z haksolot@omarchy — +scope Cargo.toml, +scope Cargo.lock, +scope src/lib.rs, +scope src/main.rs (version 1 to 2,…

BLOCKED BY (2)
├── TASK-dadb  [finished:97899fd on task/dadb] Client ank : ank <verbe> --json typé, contrat vérifié, codes de sortie routés
└── TASK-de0a  [finished:97899fd on task/de0a] Client herdr : sous-commandes CLI par $HERDR_BIN_PATH et abonnement events.subscribe par le socket

UNBLOCKS (1)
└── TASK-91a1  [open] Daemon [[startup]] : événements herdr, poll, tail de events.jsonl, une seule instance

## Mesuré le 2026-09-25 par ank-1 (ank 0.8.0)

`find --json` ne porte pas de champ `coordination` : chaque résultat porte
`state`, de la forme `"claimed:<identité>"` quand la tâche est tenue.
`coordination` n'existe que dans `show --json`. Relier un claim à une
identité se fait donc sur `state` ; l'expiration (`ank_expires`) se lit dans
`show --json` de la tâche tenue, une lecture par claim et non par pane.
Voir `ank log TASK-dadb`.
