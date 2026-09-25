---
id: LOG-3d4c879cff34
type: log
title: "CI run https://github.com/haksolot/herdr-ank/actions/runs/36178552175 on task/4e8f @27c8a98:"
created: 2026-09-25T19:17:28Z
author: haksolot@omarchy/ank-2
scope:
  - install.ps1
  - herdr-plugin.toml
  - .github/workflows/release.yml
  - tests/install_ps1.rs
  - README.md
about: TASK-4e8fd40b787a
seq: 4
schema: 4
version: 1
---

 ubuntu-latest success, macos-latest success, windows-latest failure at clippy, before any test: 'could not compile herdr-ank (lib)', E0433 cannot find unix in os at src\herdr\events.rs:5:14 and src\tui.rs:4:14, E0599 no method exec at src\tui.rs:56:46 — the same three errors TASK-6782 logged on main. So tests/install_ps1.rs cannot run on Windows until TASK-7868 (events.rs) and TASK-194a (tui.rs, tests/**) are on main; the Windows proof clause of this criterion is unreachable today. Measured instead: tests/install_ps1.rs type-checks and is clippy -D warnings clean for x86_64-pc-windows-msvc in a throwaway crate; actionlint 1.7.12 -shellcheck= reports nothing on .github/workflows/*.yml; cargo test/clippy/fmt green on linux.
