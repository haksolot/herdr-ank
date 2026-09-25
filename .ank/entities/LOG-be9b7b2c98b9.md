---
id: LOG-be9b7b2c98b9
type: log
title: "hypothesis H1: a child forked by a concurrent test inherits the descriptor carrying the flock"
created: 2026-09-25T19:41:02Z
author: haksolot@omarchy/ank-1
scope:
  - src/daemon/**
  - tests/daemon.rs
about: TASK-860ef54c4a4a
seq: 5
schema: 4
version: 1
---

 (File::try_lock) until its exec closes it (O_CLOEXEC), so the lock outlives drop(). Refuted if widening the fork->exec window leaves the rate unchanged. Measured in a std-only scratch program (thread A: take lock, sleep 5ms, drop, retake; thread B: Command::new("true") with pre_exec sleeping 50ms, in a loop): without spawns 0/200 retakes refused; with spawns 44/200 refused. H1 confirmed. Production reading: the daemon holds its Lock for its whole life and never drops-then-retakes it in-process, so the race only bites a test binary that spawns while another test round-trips a lock.
