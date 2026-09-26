---
id: TASK-ef99390bbc19
type: task
slug: action-ouvrir-ank-dans-la-palette-ouvre-l-overla
title: "Action « Ouvrir ank » dans la palette : ouvre l'overlay ank tui du workspace"
created: 2026-09-26T06:56:21Z
author: haksolot@vmi3223161
status: open
scope:
  - herdr-plugin.toml
  - src/tui.rs
  - tests/tui.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
blocked_by: []
done_criteria: |
  `herdr-plugin.toml` déclare une `[[actions]]` d'id `open`, de titre « Ouvrir ank », de contexte `workspace`, qui lance `./bin/herdr-ank open`. La sous-commande `open` lit le workspace dans `HERDR_PLUGIN_CONTEXT_JSON` et appelle `$HERDR_BIN_PATH plugin pane open --plugin ank --entrypoint tui --workspace <id>` (ADR-357c017baf9b). Sans contexte de workspace, elle sort 1 en le nommant. Un test vérifie, contre le faux binaire herdr, l'argv exact de l'appel et la sortie 1 sans contexte. `herdr plugin action list` montre l'action dans une session réelle, et un `ank log` le note.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
schema: 4
version: 1
---
