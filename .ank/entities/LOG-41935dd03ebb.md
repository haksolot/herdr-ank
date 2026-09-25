---
id: LOG-41935dd03ebb
type: log
title: "discrepancy: the criterion assumes the windows job can be green for tests/herdr_client.rs within"
created: 2026-09-25T19:17:56Z
author: haksolot@omarchy/ank-1
scope:
  - src/herdr/**
  - tests/herdr_client.rs
about: TASK-7868b3db8404
seq: 4
schema: 4
version: 1
---

 scope src/herdr/** + tests/herdr_client.rs; measured: every test target links the lib, and the lib fails on src/tui.rs (TASK-194a scope), while TASK-194a is blocked_by this task. Dependency cycle for the proof clause. Local cross-check (not proof): with a throwaway tui.rs patch, cargo clippy --target x86_64-pc-windows-msvc --test herdr_client -D warnings is clean. Remaining Windows compile errors in tests/, for TASK-194a: tests/ank_client.rs:3,49,344; tests/work.rs:7,37; tests/install.rs:6,57,176 (std::os::unix, Permissions::from_mode/.mode()).
