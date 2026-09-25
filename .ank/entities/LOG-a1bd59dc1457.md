---
id: LOG-a1bd59dc1457
type: log
title: "measured: version step run locally exits 0 on v0.1.0 and 0.1.0, exits 1 naming both versions on"
created: 2026-09-25T18:42:08Z
author: haksolot@omarchy/ank-2
scope:
  - .github/workflows/release.yml
  - tests/version.rs
  - README.md
about: TASK-a297352636ac
seq: 5
schema: 4
version: 1
---

 v0.2.0; actionlint 1.7.12 (installed via mise) with shellcheck 0.11.0 passes release.yml after replacing an ls count (SC2012) by set --; actionlint flags an injected bad output name, so the pass is meaningful; tar layout has herdr-ank at root and sha256sum -c accepts SHA256SUMS (host target); cargo build --release --locked --target x86_64-unknown-linux-musl gives a static-pie binary. No tag and no release created (gh release list empty, git tag empty); the first real run of the workflow is the human's tag
