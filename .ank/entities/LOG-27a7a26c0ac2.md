---
id: LOG-27a7a26c0ac2
type: log
title: "Lecture du critère : sans --workspace, open n'a plus besoin du workspace_id du contexte (herdr"
created: 2026-09-26T22:46:33Z
author: ank-306f
scope:
  - src/tui.rs
  - tests/tui.rs
about: TASK-306fe4707a05
seq: 1
schema: 4
version: 1
---

 ouvre l'overlay sur la pane active) ; le test « exits 1 sans workspace » devient « ouvre avec ou sans contexte ». Le client décode déjà la réponse nue {type:ok} depuis TASK-a23f ; seul tui::open passe encore --workspace.
