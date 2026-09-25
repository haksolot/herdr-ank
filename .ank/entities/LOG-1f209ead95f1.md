---
id: LOG-1f209ead95f1
type: log
title: "found on ADR-c8e7 re-read: config.sync.poll_seconds (TASK-90fb) lets the poll exceed the 30 s"
created: 2026-09-25T17:34:09Z
author: haksolot@omarchy/ank-2
scope:
  - src/sync/**
  - tests/sync.rs
about: TASK-416cdde27bfb
seq: 7
schema: 4
version: 1
---

 ADR-c8e7 requires, and the spec's fixed 90000 ttl ('three poll intervals') assumes 30 s. plan keeps the spec's literal 90000 and does not read config; a first draft derived ttl from poll_seconds, taken back out as scope nobody asked for. For planning: settle whether poll_seconds is bounded by ADR-c8e7 and whether ttl follows it
