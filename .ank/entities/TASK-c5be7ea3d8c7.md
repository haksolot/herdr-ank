---
id: TASK-c5be7ea3d8c7
type: task
slug: client-herdr-report-metadata-rapporte-sous-la-so
title: "Client herdr : report_metadata rapporte sous la source ank:sync de la spec, pas ank"
created: 2026-09-25T17:33:40Z
author: haksolot@omarchy/ank-2
status: open
scope:
  - src/herdr/**
  - tests/herdr_client.rs
blocked_by: [TASK-de0a51842971]
done_criteria: |
  `herdr::Client::report_metadata` passe `--source ank:sync` (SPEC-dbe3cf972f71) : le test d'argv de `tests/herdr_client.rs` attend `ank:sync`, et aucune occurrence de la source `ank` nue ne reste dans `src/herdr/`. Les autres tests du client restent verts.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
schema: 4
version: 1
---

Trouvé en travaillant TASK-416c : TASK-de0a a livré `SOURCE = "ank"` dans src/herdr/mod.rs, alors que SPEC-dbe3 impose `--source ank:sync` et interdit toute autre source au daemon. La spec ne couvre que src/sync/**, si bien que `ank context src/herdr` ne la montrait pas au moment de TASK-de0a. `sync::Report` porte déjà `source: "ank:sync"` ; le plus simple est que le client prenne cette valeur (constante ou paramètre), à trancher dans la tâche.
