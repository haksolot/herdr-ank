---
id: LOG-2e8ec46eb45d
type: log
title: "review, criterion axis: build->target/release/herdr-ank (Cargo.toml+src/main.rs); --version prints"
created: 2026-09-25T17:15:18Z
author: haksolot@omarchy/ank-1
scope:
  - Cargo.toml
  - herdr-plugin.toml
  - src/main.rs
  - .gitignore
about: TASK-5dbc8dd1aebf
seq: 2
schema: 4
version: 1
---

 0.1.0 from Cargo.toml (clap version); herdr-plugin.toml id ank, name, version, min_herdr_version 0.9.1, platforms [linux,macos], [[build]] cargo build --release; herdr plugin link . exit 0, plugin list --json shows ank enabled:true (linked at this worktree path). Body's daemon/sync/work/tui stubs exit 1. Extra: description keys (metadata only), /target in .gitignore; Cargo.lock generated outside scope, committed since herdr-ank is a binary. Constraint axis (ank context on the 4 touched paths): ADR-aca6 held (single Rust binary, only manifest entry is the cargo build, linux+macos, no windows); ADR-357c/3cd1/fa8b not exercised by a stub, nothing found.
