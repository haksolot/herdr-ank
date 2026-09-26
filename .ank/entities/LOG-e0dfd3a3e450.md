---
id: LOG-e0dfd3a3e450
type: log
title: "src/land : land(&Request) -> Result<Landing, LandError>. L'état ank de la tâche est le Found d'ank"
created: 2026-09-26T10:09:03Z
author: haksolot@vmi3223161/ank-a4cf
scope:
  - src/land/**
  - tests/land.rs
  - tests/support/**
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-a4cfb858a246
seq: 1
schema: 4
version: 1
---

 find --json (Show n'a pas de champ status). Worktree hors de sa branche = LandError::WorktreeNotOnBranch (hors liste de refus, lu avant toute écriture). Tests sur vrais dépôts : main qui avance entre rebase et ff simulé par un hook post-rewrite partagé qui committe dans l'arbre d'intégration, N fois. Tous rouges sur assertion avant l'implémentation ; le test de garde ADR-ff45 (grep des verbes git) vu rouge par injection d'un git branch. remedy/Display des refus laissés à TASK-a23f.
