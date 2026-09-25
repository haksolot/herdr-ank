---
id: LOG-54f5eb4e3fb7
type: log
title: "PROOF: CI run https://github.com/haksolot/herdr-ank/actions/runs/36180142206 on task/194a at"
created: 2026-09-25T19:34:50Z
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
seq: 4
schema: 4
version: 1
---

 039ce27a8ee3: attempt 1 windows-latest success, macos-latest success, ubuntu-latest failure on tests/daemon.rs:37 a_second_lock_on_the_state_dir_is_refused_until_the_first_is_dropped (file untouched by this diff; flock shared by a fork between open and exec, suspected); attempt 2 (rerun of the failed job, same commit): ubuntu-latest success. All three jobs (fmt, clippy -D warnings, test --locked) green on 039ce27. Flake filed as TASK-860e (method diagnose).
