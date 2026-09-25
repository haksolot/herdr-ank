---
id: TASK-c81aba215f4d
type: task
slug: action-travailler-une-t-che-choisir-cr-er-le-wor
title: "Action « travailler une tâche » : choisir, créer le worktree, démarrer l'agent avec son identité, faire claim"
created: 2026-09-25T16:54:28Z
author: haksolot@omarchy
status: open
scope:
  - src/work/**
  - src/pick.rs
  - herdr-plugin.toml
  - tests/work.rs
blocked_by: [TASK-dadb4d2261b5, TASK-de0a51842971, TASK-90fb516a9fcf]
done_criteria: |
  `herdr-plugin.toml` déclare `[[actions]] id = "work"`, contexte `workspace`, commande `herdr-ank work`, et `[[panes]] id = "pick"` en `popup`. `herdr-ank work` ouvre la pane `pick` qui liste les tâches claimables du corpus du workspace (`context --json`, ni tenues ni bloquées), filtrables au clavier ; la sélection écrit l'id dans `HERDR_PLUGIN_STATE_DIR`. `work` enchaîne alors : `worktree create --branch task/<id court> --base <default_branch>`, `tab create --cwd <worktree> --env ANK_AGENT=<user>@<host>/<nom>` où `<nom>` est `ank-<id court>`, `agent start <nom> --kind <config> --pane <pane de l'onglet>`, puis `agent prompt <nom> "ank claim <id>"`. Si le claim échoue avec le code 4, l'utilisateur est notifié et le worktree n'est pas supprimé. Le test couvre l'ordre exact des appels herdr sur le binaire factice, et le refus propre quand le workspace n'a pas de corpus.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
schema: 4
version: 1
---

La fonction qui applique « un agent, un arbre, une identité » sans que
l'utilisateur tape une commande (ADR-fa8b6a103597).

Le picker est un TUI minimal du binaire lui-même (crossterm ou ratatui) :
herdr v1 n'a pas d'UI native, et `ank tui` compose des commandes mais ne rend
pas de sélection à un tiers. Le popup est singleton et modal, ce qui convient.

`default_branch` vient de `ank status --json` ; s'il est null le worktree
part de HEAD et le journal le dit. Le nom `ank-<id court>` est proposé par
défaut ; un `herdr agent rename` ultérieur casse l'attribution et
ADR-fa8b6a103597 l'assume.
