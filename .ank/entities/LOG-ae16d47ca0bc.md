---
id: LOG-ae16d47ca0bc
type: log
title: "CI run 36177823391 on task/6782 (commit 9b9293c): ubuntu-latest success, macos-latest success,"
created: 2026-09-25T19:12:08Z
author: haksolot@omarchy/ank-1
scope:
  - .github/workflows/ci.yml
  - AGENTS.md
  - README.md
about: TASK-678211d8548c
seq: 2
schema: 4
version: 1
---

 windows-latest failure. Windows: cargo fmt --check passes; cargo clippy --workspace --all-targets fails to compile the lib (so cargo test never ran), exactly 3 errors: (1) E0433 src/herdr/events.rs:5 use std::os::unix::net::UnixStream -> TASK-7868; (2) E0433 src/tui.rs:4 use std::os::unix::process::CommandExt; (3) E0599 src/tui.rs:56 Command::exec not found -> TASK-194a. Test targets were not reached: errors in tests/ (sh fake binaries, std::os::unix in tests/install.rs symlink etc.) and HOME-rooted worktree path will only surface once the lib compiles; the porting tasks must re-run CI to see them.
