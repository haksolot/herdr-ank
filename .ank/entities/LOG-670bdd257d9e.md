---
id: LOG-670bdd257d9e
type: log
title: "discrepancy: body assumes find --status in_progress --json carries coordination per result;"
created: 2026-09-25T17:17:46Z
author: haksolot@omarchy/ank-1
scope:
  - src/ank/**
  - tests/ank_client.rs
  - tests/fixtures/ank/**
about: TASK-dadb4d2261b5
seq: 1
schema: 4
version: 1
---

 measured (ank 0.8.0 298c01a, help --json and live) find results carry state "claimed:<holder>", coordination is only on show. The client types state; mapping claim->identity downstream (TASK-416c) should parse state.
