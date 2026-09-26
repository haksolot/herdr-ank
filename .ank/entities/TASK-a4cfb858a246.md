---
id: TASK-a4cfb858a246
type: task
slug: module-land-pr-conditions-rebase-et-fast-forward
title: "Module land : préconditions, rebase et fast-forward d'une branche de tâche, sans rien nettoyer"
created: 2026-09-26T09:47:36Z
author: haksolot@vmi3223161
status: open
scope:
  - src/land/**
  - tests/land.rs
  - tests/support/**
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
blocked_by: []
done_criteria: |
  src/land expose une fonction qui prend l'arbre d'intégration, le worktree de la tâche, sa branche task/<id court>, la branche par défaut et l'état ank de la tâche, et rend un résultat typé : Landed, ou un refus parmi NotDone, DirtyWorktree, IntegrationNotOnDefault, DirtyIntegration, RebaseConflict, FastForwardRefused. Toutes les préconditions (statut done lu par ank --json, worktree propre, arbre d'intégration propre et sur la branche par défaut) sont vérifiées avant toute écriture. Puis git -C <worktree> rebase <défaut> ; en cas de conflit, rebase --abort et RebaseConflict, la branche étant revenue à son commit d'avant. Puis git -C <intégration> merge --ff-only task/<id court> ; si refusé, un seul nouveau rebase puis un seul nouvel essai avant FastForwardRefused. Seules les opérations git d'ADR-ff4569057dc5 apparaissent dans src/land. tests/land.rs construit de vrais dépôts git temporaires (arbre d'intégration + worktree) et couvre : le landing nominal (la branche par défaut pointe sur le sommet de la branche de tâche), chaque refus avec l'état git inchangé, le conflit annulé, et le main qui avance entre rebase et fast-forward (réussi au second essai). Les tests passent sur ubuntu, macos et windows en CI.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
schema: 4
version: 1
---

Le cœur du landing, séparé de l'action et du nettoyage pour être testé contre git seul. Règles : ADR-ff4569057dc5 (proposée). Aucun appel herdr ici, aucune suppression de branche ni de worktree : c'est la tâche de l'action.
