---
id: LOG-ffdcce34b4f9
type: log
title: "PROOF: CI run https://github.com/haksolot/herdr-ank/actions/runs/36180638287 on task/7868 at"
created: 2026-09-25T19:37:53Z
author: haksolot@omarchy/ank-1
scope:
  - src/herdr/**
  - tests/herdr_client.rs
about: TASK-7868b3db8404
seq: 9
schema: 4
version: 1
---

 0cd3ddd (main dde3670 + claim): ubuntu-latest, macos-latest, windows-latest all success, first attempt. Windows job, tests/herdr_client.rs: 26 passed, among them subscribe_decodes_the_fixed_stream_and_skips_the_unknown_type, subscribe_decodes_the_lifecycle_events_the_daemon_wakes_on (in-memory NDJSON replay) and subscribe_to_an_absent_named_pipe_is_a_herdr_error_naming_the_pipe (cfg(windows)); the Unix-socket test is cfg(unix) and absent there.
