---
id: TASK-5dcff7b9d21b
type: task
slug: ank-tui-passe-par-src-ank-sans-prendre-l-identit
title: ank tui passe par src/ank sans prendre l'identité herdr-ank
created: 2026-09-25T18:00:02Z
author: haksolot@omarchy/ank-1
status: open
scope:
  - src/tui.rs
  - src/ank/**
  - tests/ank_client.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
blocked_by: []
done_criteria: |
  La commande `ank tui` que `herdr-ank tui` exécute est construite par une fonction de `src/ank/` (`ank::tui_command(repo)`), avec le corpus comme répertoire courant et sans `--repo`, comme aujourd hui ; elle ne pose pas `ANK_AGENT` et laisse l environnement de l utilisateur intact. `src/tui.rs` ne contient plus de `Command::new("ank")`. Un test vérifie le programme, les arguments, le répertoire courant et l absence de `ANK_AGENT` posé sur la commande construite.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
schema: 4
version: 1
---

Découvert en prenant TASK-56c4 : son critère exclut tout `Command::new("ank")` hors de `src/ank/`, et `src/tui.rs:67` (TASK-8e7f) en porte un, hors du scope de 56c4. Le TUI d ank est celui de l humain : un claim fait depuis lui doit garder son identité, jamais `<user>@<host>/herdr-ank` que 56c4 donne aux lectures du plugin. Cette tâche déplace la construction sans changer le comportement ; 56c4 peut ensuite exclure `tui_command` de l identité herdr-ank.
