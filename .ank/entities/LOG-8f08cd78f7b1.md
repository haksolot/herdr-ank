---
id: LOG-8f08cd78f7b1
type: log
title: "self-review: criterion axis — cannot fail through a fork sharing the locked descriptor: Drop"
created: 2026-09-25T19:46:31Z
author: haksolot@omarchy/ank-1
scope:
  - src/daemon/**
  - tests/daemon.rs
about: TASK-860ef54c4a4a
seq: 7
schema: 4
version: 1
---

 unlocks the open file description itself, which a duplicate cannot keep alive (0/1000); cause established by reproduction before any change (logs: 12/1000, --skip 0/1000, scratch 44/200 vs 0/200); regression test seen red without the fix; 50 consecutive passes. No hunk beyond: one Drop impl, one field rename it needs, one test. Constraint axis (ank context src/daemon/mod.rs): ADR-6fb7 single instance locked in HERDR_PLUGIN_STATE_DIR — the lock is still held for the daemon's life and released at exit; unlock on drop only frees it sooner, never earlier than the Lock's owner lets go. ADR-599b: File::unlock is std on all three OSes, no cfg needed. ADR-357c, 3cd1, fa8b untouched. Nothing found.
