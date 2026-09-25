---
id: LOG-b9b0b758dd20
type: log
title: "discrepancy: scope lists src/ank/**, tests/ank_client.rs, tests/fixtures/ank/** but the criterion"
created: 2026-09-25T17:21:25Z
author: haksolot@omarchy/ank-1
scope:
  - src/ank/**
  - tests/ank_client.rs
  - tests/fixtures/ank/**
about: TASK-dadb4d2261b5
seq: 4
schema: 4
version: 1
---

 cannot be met inside it: deserializing needs serde+serde_json in Cargo.toml (+Cargo.lock), and tests/ can only reach a module of a library target, so src/lib.rs (pub mod ank;) was added. No live claim held any of the three; the hunks are the minimum (2 dep lines, 1 lib line). TASK-de0a (serde_json) and TASK-90fb (toml) will need the same Cargo.toml/src/lib.rs edits: planning should add them to those scopes.
