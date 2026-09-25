---
id: TASK-860ef54c4a4a
type: task
slug: le-test-du-verrou-du-daemon-ne-d-pend-plus-d-un
title: Le test du verrou du daemon ne dépend plus d'un fork concurrent
created: 2026-09-25T19:34:12Z
author: haksolot@omarchy/ank-1
status: open
scope:
  - src/daemon/**
  - tests/daemon.rs
blocked_by: []
done_criteria: |
  Le test a_second_lock_on_the_state_dir_is_refused_until_the_first_is_dropped ne peut plus échouer parce qu'un autre test du même binaire forke un enfant qui partage le descripteur verrouillé entre fork et exec ; la cause est établie par une reproduction (par exemple le test lancé en boucle aux côtés des tests qui spawnent herdr-ank) avant tout changement, et un test de régression la couvre. cargo test --test daemon passe 50 fois de suite sur Linux.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: diagnose
schema: 4
version: 1
---

Découvert pendant TASK-194a : run https://github.com/haksolot/herdr-ank/actions/runs/36180142206 tentative 1, ubuntu-latest rouge sur tests/daemon.rs:37 « a dropped lock is free », vert en tentative 2 sans changement. Lock::acquire prend un flock (File::try_lock) sur une description de fichier ouverte ; les autres tests de daemon.rs spawnent herdr-ank, et un enfant forké entre l'ouverture et l'exec hérite du descripteur (fermé seulement à l'exec par O_CLOEXEC), donc le verrou survit un instant au drop. Hypothèse à confirmer, pas une cause établie.
