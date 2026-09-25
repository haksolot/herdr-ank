---
id: LOG-a65bf00a2e93
type: log
title: "fix: src/daemon/mod.rs Lock gets Drop calling File::unlock before the close (field _file -> file);"
created: 2026-09-25T19:46:29Z
author: haksolot@omarchy/ank-1
scope:
  - src/daemon/**
  - tests/daemon.rs
about: TASK-860ef54c4a4a
seq: 6
schema: 4
version: 1
---

 nothing else changed. Regression test tests/daemon.rs a_dropped_lock_is_free_while_another_test_spawns_processes (derived from the minimal case: 2000 lock round trips while a thread spawns herdr-ank in a loop, counting refusals): without the fix red 10/10 (e.g. 'held after its drop 104/2000 times'), then 4/5 on the final version with the unlock commented out; with the fix 0/20 red. Measurement of the criterion: cargo test --test daemon 50/50 consecutive passes; daemon test binary run 1000 times: 0/1000 failures (12/1000 before). CI https://github.com/haksolot/herdr-ank/actions/runs/36181393620 at 190da6a: ubuntu, macos, windows green.
