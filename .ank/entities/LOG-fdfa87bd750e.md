---
id: LOG-fdfa87bd750e
type: log
title: "review, axis criterion: manifest has one [[events]] per workspace.created, tab.created,"
created: 2026-09-25T18:17:16Z
author: haksolot@omarchy/ank-2
scope:
  - herdr-plugin.toml
  - src/daemon/**
  - src/main.rs
  - tests/daemon.rs
  - README.md
about: TASK-d9b1469fe725
seq: 6
schema: 4
version: 1
---

 pane.agent_detected, worktree.opened, each './target/release/herdr-ank daemon' (herdr link lists all four); with HERDR_PLUGIN_EVENT=tab.created a held lock exits 0 with empty stdout and stderr, a free lock makes the process the daemon (tests); live: one process within 10 s of tab create after link, a new one within 10 s after kill, one only after two close tab creates (also measured with no daemon alive); README 'Daemon lifecycle' describes the cycle. Nothing unasked beyond the README correction the measurement called for. Axis constraints: ADR-6fb7 (single process locked in the state dir, silent exit 0 while held, relaunch by [[events]], no external supervisor) held; ADR-aca6 manifest entries run the release binary; herdr server never stopped
