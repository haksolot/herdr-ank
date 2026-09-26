---
id: LOG-bccdaca6f909
type: log
title: "Session herdr réelle mesurée (herdr 0.9.1). Avant : l'action Ouvrir ank du plugin installé (release"
created: 2026-09-26T22:48:49Z
author: ank-306f
scope:
  - src/tui.rs
  - tests/tui.rs
about: TASK-306fe4707a05
seq: 3
schema: 4
version: 1
---

 5428d22) échoue, plugin-log-57 exit 1 « herdr invalid_params: overlay and popup plugin panes target the active pane ». Après : le binaire de cette branche, lancé comme l'action avec le HERDR_PLUGIN_CONTEXT_JSON que herdr lui a passé (workspace_id w3A), sort 0 ; l'overlay w3A:pB label « Ank » s'ouvre, focused, au-dessus de la pane active du workspace w3A, et affiche ank tui sur le corpus herdr-ank (liste des tâches). Refermée ensuite par plugin pane close.
