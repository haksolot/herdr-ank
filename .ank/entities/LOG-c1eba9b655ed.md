---
id: LOG-c1eba9b655ed
type: log
title: "minimise: lock test alone --exact: 0/500. Lock test + any single spawning test (--exact pairs):"
created: 2026-09-25T19:41:00Z
author: haksolot@omarchy/ank-1
scope:
  - src/daemon/**
  - tests/daemon.rs
about: TASK-860ef54c4a4a
seq: 4
schema: 4
version: 1
---

 0/300 each. Full binary with --skip the_daemon_exits_zero --skip an_event_hook_finding (the 3 tests that spawn herdr-ank): 0/1000; full binary: 12/1000. The spawning tests are necessary to the failure.
