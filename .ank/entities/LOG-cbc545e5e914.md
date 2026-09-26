---
id: LOG-cbc545e5e914
type: log
title: "Troisième constat en session réelle : une tâche finie sur sa branche est status done vue du"
created: 2026-09-26T11:20:37Z
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
seq: 9
schema: 4
version: 1
---

 worktree, mais status open, state « finished:<sha> on task/<id> » vue de l'arbre d'intégration, tant qu'elle n'est pas landée ; find --status done sur le corpus ne la liste donc jamais. Lecture retenue de « tâches done » : status done ou state finished:. L'action lit la tâche par ank --repo <worktree>, où elle est done, avant les préconditions de land.
