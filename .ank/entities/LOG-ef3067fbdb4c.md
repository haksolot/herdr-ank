---
id: LOG-ef3067fbdb4c
type: log
title: "Défaut mesuré en session réelle (herdr 0.9.1) : plugin pane open --workspace <w> sur une pane popup"
created: 2026-09-26T10:32:56Z
author: haksolot@vmi3223161/ank-a23f
scope:
  - herdr-plugin.toml
  - src/land/**
  - src/herdr/**
  - src/pick.rs
  - tests/land_action.rs
  - tests/herdr_client.rs
  - tests/support/**
  - README.md
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
  - tests/land.rs
  - src/work/mod.rs
  - tests/work.rs
about: TASK-a23f62557764
seq: 6
schema: 4
version: 1
---

 est refusé, invalid_params « overlay and popup plugin panes target the active pane » ; sans --workspace il réussit. work passait --workspace, donc Work a task échoue à la popup dans herdr réel, et land ferait de même. Scope étendu à src/work/mod.rs et tests/work.rs : work et land ouvrent la popup par pick::choose, sans --workspace, testé d'abord (l'argv attendu perd --workspace).
