---
id: LOG-6c601cefa1d9
type: log
title: "discrepancy: herdr.dev/docs/plugins/ calls [[startup]] hooks 'one-shot initialization commands"
created: 2026-09-25T17:48:31Z
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
seq: 10
schema: 4
version: 1
---

 rather than supervised daemons', started asynchronously with completion logged; it says nothing of a timeout or kill. ADR-c8e7 and this criterion assume a long-lived process from [[startup]]: implemented as asked (nothing measured refutes it), but nothing supervises or restarts the daemon if it dies. For planning: verify live that herdr leaves a long [[startup]] command running, and decide who restarts it
