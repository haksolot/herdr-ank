---
id: LOG-219c1bbbc388
type: log
title: "Workspace tokens : sync::plan_workspaces (majorité de panes par workspace, égalité à la première"
created: 2026-09-26T08:00:40Z
author: haksolot@vmi3223161/ank-ef99
scope:
  - src/sync/**
  - src/daemon/**
  - tests/sync.rs
  - tests/daemon.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-c63d04056506
seq: 2
schema: 4
version: 1
---

 pane de pane list), appelé par sync_once qui lit désormais le corpus de toute pane, agent ou non. ank_review vient de la map status.queue par racine, passée en paramètre (un champ sur Corpus aurait cassé tests/notify.rs, hors scope). La notification « N décisions en attente » garde son périmètre : ses files sont restreintes aux corpus des panes d'agent. Rouges vus : 4 tests plan_workspaces sur stub vide, puis aucun appel workspace report-metadata dans le test daemon.
