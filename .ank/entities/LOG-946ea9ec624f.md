---
id: LOG-946ea9ec624f
type: log
title: "daemon clauses 1-4: tests/daemon.rs gains 4 tests (first start writes key+2 sidebars on a config"
created: 2026-09-27T07:34:43Z
author: haksolot@omarchy/ank-7325
scope:
  - src/daemon/mod.rs
  - src/setup.rs
  - tests/daemon.rs
  - tests/setup.rs
  - README.md
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-7325c78f7906
seq: 2
schema: 4
version: 1
---

 with prefix=ctrl+space; second start after blocks removed: 1 config check, 1 reload total, config untouched; prefix+a->lazygit: 1 notification naming prefix+a and herdr-ank setup, first sync still reached; check refused: config restored, 0 reload, 1 notification, not retried). Red first: 4 FAILED / 21 passed (no write, 0 check). Green after setup::apply + daemon configure_herdr, marker setup-tried: 25 passed. Every test daemon now gets HERDR_CONFIG_PATH beside its state dir so no test can reach the user's config.
