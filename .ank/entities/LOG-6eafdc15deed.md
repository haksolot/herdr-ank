---
id: LOG-6eafdc15deed
type: log
title: "reproduce: target/debug/deps/daemon-<hash> -q run in a loop (Linux, cargo test --test daemon"
created: 2026-09-25T19:40:57Z
author: haksolot@omarchy/ank-1
scope:
  - src/daemon/**
  - tests/daemon.rs
about: TASK-860ef54c4a4a
seq: 3
schema: 4
version: 1
---

 --no-run build): 3/200 then 12/1000 runs fail, always a_second_lock_on_the_state_dir_is_refused_until_the_first_is_dropped at tests/daemon.rs:37 'a dropped lock is free' (1 of 400 captured runs, same test and line).
