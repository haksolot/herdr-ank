---
id: LOG-57186d4e6fcb
type: log
title: "Scope étendu à tests/land.rs : la garde d'ADR-ff45 posée par TASK-a4cf n'admettait que status,"
created: 2026-09-26T10:30:25Z
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
about: TASK-a23f62557764
seq: 4
schema: 4
version: 1
---

 rev-parse, rebase, merge et interdisait "push" ; le nettoyage a besoin de merge-base --is-ancestor, ls-remote, branch -d et push origin --delete, que l'ADR liste. La garde est élargie à exactement cette liste, push seulement sous la forme push origin --delete, branch seulement -d.
