---
id: LOG-9fe68a2c26d8
type: log
title: "red/green: a hook finding the lock held printed 'another instance holds the lock' on stderr, which"
created: 2026-09-25T18:14:49Z
author: haksolot@omarchy/ank-2
scope:
  - herdr-plugin.toml
  - src/daemon/**
  - src/main.rs
  - tests/daemon.rs
  - README.md
about: TASK-d9b1469fe725
seq: 3
schema: 4
version: 1
---

 every [[events]] spawn would add to herdr's plugin log; now silent (test red first on that line). A hook finding the lock free already became the daemon (green on its first run: behaviour from TASK-91a1, the test now pins it with HERDR_PLUGIN_EVENT=tab.created). The daemon does not branch on HERDR_PLUGIN_EVENT at all, so startup and every event kind behave alike
