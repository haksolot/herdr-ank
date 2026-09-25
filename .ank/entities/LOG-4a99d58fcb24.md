---
id: LOG-4a99d58fcb24
type: log
title: "review. criterion axis: on.push.tags v* -> clause 2; job version -> 3; matrix of four targets"
created: 2026-09-25T18:42:11Z
author: haksolot@omarchy/ank-2
scope:
  - .github/workflows/release.yml
  - tests/version.rs
  - README.md
about: TASK-a297352636ac
seq: 6
schema: 4
version: 1
---

 (native runners, ubuntu-24.04-arm for aarch64 musl, macos-latest for both darwin) -> 4; tar -C ... herdr-ank -> 5; sha256sum over the four, count asserted -> 6; gh release create v<version> --verify-tag with the four archives + SHA256SUMS -> 7; actionlint -> 8; tests/version.rs cargo_and_manifest_carry_the_same_version -> 1 (red watched by bumping the manifest temporarily), release_workflow_builds_the_archives_install_sh_expects is the test for 4-6 and pins the names install.sh (TASK-5302) relies on. No unasked hunk; README untouched. constraints axis (ank context on release.yml, tests/version.rs): ADR-7aab — tag-triggered, refuses divergence from Cargo.toml or manifest, linux musl + macos x86_64/aarch64, no Windows; nothing found against it
