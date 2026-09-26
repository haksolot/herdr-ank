---
id: TASK-4fbfdb3b3a55
type: task
slug: client-herdr-report-metadata-porte-display-agent
title: "Client herdr : report_metadata porte --display-agent, et un rapport de workspace par workspace report-metadata ; mesure de --display-agent dans herdr 0.9.1"
created: 2026-09-26T06:55:41Z
author: haksolot@vmi3223161
status: open
scope:
  - src/herdr/mod.rs
  - tests/herdr_client.rs
  - tests/support/fake_cli.rs
  - tests/support/mod.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
blocked_by: []
done_criteria: |
  Le client herdr (src/herdr/mod.rs) passe `--display-agent <texte>` à `pane report-metadata` quand on lui en donne un, et offre `report_workspace_metadata(workspace, tokens, clear, ttl)`, qui appelle `workspace report-metadata <id> --source ank:sync [--token N=V]... [--clear-token N]... [--ttl-ms N]` par $HERDR_BIN_PATH (ADR-357c017baf9b). tests/herdr_client.rs vérifie, contre le faux binaire de tests/support, l'argv exact des deux appels, avec et sans libellé. Mesure, dans une session herdr 0.9.1 réelle, sur une pane où tourne un agent : un `ank log` sur cette tâche note (a) si le champ `agent` de `herdr pane list` reste le nom détecté après un rapport `--display-agent "claude · ank"`, (b) si la ligne `agent` par défaut de la sidebar affiche ce libellé, (c) si le libellé d'origine revient une fois le ttl écoulé. Chaque point cite la sortie observée ou le code de herdr (github.com/herdrdev/herdr). Si (a) est faux, le log le dit, car l'attribution d'ADR-fa8b6a103597 casserait, et la tâche s'arrête là, sans rien câbler dans la sync.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
schema: 4
version: 2
---
