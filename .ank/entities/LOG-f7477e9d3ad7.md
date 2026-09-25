---
id: LOG-f7477e9d3ad7
type: log
title: "review, criterion axis: no Command::new(\"ank\") outside src/ank (test"
created: 2026-09-25T18:04:33Z
author: haksolot@omarchy/ank-1
scope:
  - src/ank/**
  - src/daemon/**
  - tests/daemon.rs
  - tests/ank_client.rs
about: TASK-56c4d26c623f
seq: 7
schema: 4
version: 1
---

 nothing_outside_the_ank_module_spawns_ank walks src/; daemon's events_jsonl moved to ank::events_jsonl); every client verb carries ANK_AGENT=<user>@<host>/herdr-ank (Client::command, tests on the built command's env and on the env the fake ank received, probe once per client, suffix stripped); context stays in read mode - measured live on this repo: identity haksolot@omarchy/herdr-ank, no claim, context mode orientation, ready 1, while this pane's ank-1 identity holds a claim; sync::plan test for a bare <user>@<host> claim with a non-empty queue (tests/daemon.rs, ank_task and ank_queue=3). Red: missing symbols (compile), the src/ walk failing on src/daemon before the move, then mutations (no env on verbs, probe keeping ANK_AGENT, no cache, suffix kept) each failed. Constraint axis (ank context on src/ank, src/daemon, tests): ADR-3cd1 held and tightened - only src/ank spawns ank, contract and exit code routing unchanged (read/check factored out of run); ADR-fa8b - identity form <user>@<host>/<name> with a name no work agent carries (ank-*); ADR-c8e7 cadence untouched; ADR-357c/aca6 untouched.
