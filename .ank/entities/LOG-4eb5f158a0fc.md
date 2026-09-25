---
id: LOG-4eb5f158a0fc
type: log
title: "CI run https://github.com/haksolot/herdr-ank/actions/runs/36179907264 (e8c2853): ubuntu/macos"
created: 2026-09-25T19:34:48Z
author: haksolot@omarchy/ank-1
scope:
  - src/tui.rs
  - src/main.rs
  - src/ank/**
  - src/work/**
  - tests/**
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
about: TASK-194a3122f31c
seq: 3
schema: 4
version: 1
---

 green; windows red on one test only, work_chains_the_herdr_calls_in_order_after_the_pick: the test expected join("worktrees/repo/ank-c81a") as one string while the code joins per component (backslashes on Windows). Test fixed to join per component (039ce27). tests/tui.rs already green on windows at that run.
