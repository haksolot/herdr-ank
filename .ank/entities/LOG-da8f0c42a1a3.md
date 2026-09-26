---
id: LOG-da8f0c42a1a3
type: log
title: Session herdr réelle mesurée (herdr 0.9.1, binaire de cette branche, dépôt jetable land-exp-repo
created: 2026-09-26T12:20:09Z
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
seq: 11
schema: 4
version: 1
---

 avec corpus et origin nu) : herdr-ank work a ouvert la popup, créé le worktree ~/.herdr/worktrees/land-exp-repo/ank-2244 (workspace propre w3P) et l'onglet w3N:t2 (agent de kind pi volontairement absent : agent start a expiré, le reste est ce que Work laisse) ; tâche TASK-2244 claim, commit, ank done, poussée sur origin. herdr-ank land : Landed { head 0c17bc3, failures: [] }, notification « TASK-2244 landée sur main ». Après : aucune pane sous le worktree (w3N:t2 fermé, w3P disparu avec worktree remove), worktree list ne donne que main, git worktree list idem, le répertoire n'existe plus, branche locale task/2244 absente, ls-remote origin ne donne que main ; la tâche est done sur main. Les popups ont reçu leur choix par le fichier de sélection : le plugin installé (release) sert encore l'entrée pick.
