---
id: LOG-c3dd1a085ebe
type: log
title: "clauses: (1) tests/version.rs: Cargo.toml version == herdr-plugin.toml version; (2) release.yml on"
created: 2026-09-25T18:39:10Z
author: haksolot@omarchy/ank-2
scope:
  - .github/workflows/release.yml
  - tests/version.rs
  - README.md
about: TASK-a297352636ac
seq: 4
schema: 4
version: 1
---

 tags v*; (3) first job fails when tag minus v differs from either version; (4) matrix of the four targets; (5) archive herdr-ank-<version>-<target>.tar.gz with herdr-ank at root; (6) SHA256SUMS over the four; (7) release v<version> with five files; (8) actionlint, else yaml parse; no tag, no release made here. Reading: README is in scope but no clause asks for it; left untouched (overlap with TASK-5302)
