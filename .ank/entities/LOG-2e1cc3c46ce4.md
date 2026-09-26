---
id: LOG-2e1cc3c46ce4
type: log
title: "Choix : src/daemon/mod.rs (hors scope) appelle report_metadata(pane, tokens, clear, ttl) ; sa"
created: 2026-09-26T07:03:09Z
author: haksolot@vmi3223161/ank-4fbf
scope:
  - src/herdr/mod.rs
  - tests/herdr_client.rs
  - tests/support/fake_cli.rs
  - tests/support/mod.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-4fbfdb3b3a55
seq: 5
schema: 4
version: 1
---

 signature reste, et report_metadata_as(pane, display_agent: Option<&str>, tokens, clear, ttl) porte le libellé, report_metadata y délègue avec None. --display-agent est placé juste après --source. report_workspace_metadata(workspace, tokens, clear, ttl) sous le même METADATA_SOURCE ank:sync ; argv vérifié en vrai sur w3A sous ank:measure, puis effacé.
