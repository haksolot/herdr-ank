---
id: LOG-afa4ae1aa88e
type: log
title: "released: blocked: ADR-c8e7 needs a sync on pane.agent_detected, worktree.created/opened/removed"
created: 2026-09-25T17:45:19Z
author: haksolot@omarchy/ank-2
scope:
  - src/daemon/**
  - herdr-plugin.toml
  - tests/daemon.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-91a148032c7e
seq: 7
schema: 4
version: 1
---

 and workspace.closed, but herdr::subscribe drops those kinds (only pane events are decoded) and src/herdr/** is outside this scope. Filed the decoding as a new task, 91a1 now blocked_by it; taking it next, then back here
