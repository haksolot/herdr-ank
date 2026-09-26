---
id: LOG-cdabbdf80aeb
type: log
title: "Libellé posé par sync::plan (Report.display_agent) : <agent> est le champ agent de pane list"
created: 2026-09-26T07:56:31Z
author: haksolot@vmi3223161/ank-ef99
scope:
  - src/sync/**
  - src/daemon/**
  - tests/sync.rs
  - tests/daemon.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-68b31143b648
seq: 1
schema: 4
version: 1
---

 (pane.agent), l'attribution lit toujours le name de agent list. Pane sans champ agent : pas de libellé (lecture de « sans agent »). Test daemon : j'ai implémenté herdr-ank sync (une passe de daemon::sync_once, qui renvoyait « not implemented yet ») pour tester l'argv au niveau du binaire avec PATH posé sur le seul enfant. Rouges vus : champ absent, puis None au lieu du libellé, puis argv sans --display-agent.
