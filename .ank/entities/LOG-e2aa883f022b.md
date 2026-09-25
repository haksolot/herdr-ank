---
id: LOG-e2aa883f022b
type: log
title: "review, criterion axis: agent_start(name, kind, pane, args) appends '-- <arg>...' only when args is"
created: 2026-09-25T17:58:40Z
author: haksolot@omarchy/ank-1
scope:
  - src/work/**
  - src/herdr/**
  - tests/work.rs
  - tests/herdr_client.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-3513e2275e4b
seq: 4
schema: 4
version: 1
---

 non-empty (src/herdr/mod.rs), argv checked with and without args (tests/herdr_client.rs, an arg that looks like a flag stays one argument); work passes config.agent.args (src/work/mod.rs, tests/work.rs the_agent_args_come_from_the_config); README row now 'extra arguments passed to that agent, after --'. Red: compile error on the new arity, then mutations (no '--', args dropped in work) each failed their test. Nothing beyond the criterion. Constraint axis (ank context on src/herdr, src/work, tests, README): ADR-357c held - argv to $HERDR_BIN_PATH, no shell; ADR-aca6/3cd1/fa8b untouched.
