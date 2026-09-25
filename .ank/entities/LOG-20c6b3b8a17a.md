---
id: LOG-20c6b3b8a17a
type: log
title: "live run against herdr 0.9.1 (temporary state/config dirs, 15 s): lock refused a second instance"
created: 2026-09-25T17:51:03Z
author: haksolot@omarchy/ank-2
scope:
  - src/daemon/**
  - herdr-plugin.toml
  - tests/daemon.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-91a148032c7e
seq: 12
schema: 4
version: 1
---

 (exit 0); first sync put ank_task=TASK-91a1, ank_title truncated at 60, ank_expires=61, ank_queue on this pane, ank_queue alone on the other agent panes, nothing on the agentless pane. Defect found: report-metadata succeeds with empty stdout and the client calls it Decode; filed as TASK (src/herdr/**), the daemon only logs it per report and keeps going. Also: the daemon now drops ANK_AGENT like work does, so ank answers as the bare identity
