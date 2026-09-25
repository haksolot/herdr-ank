---
id: LOG-7ad2cab023ef
type: log
title: "review. Criterion axis: smoke job needs [version, release], matrix ubuntu/macos/windows-latest,"
created: 2026-09-25T20:03:05Z
author: haksolot@omarchy/ank-2
scope:
  - .github/workflows/release.yml
  - Cargo.toml
  - Cargo.lock
  - herdr-plugin.toml
  - README.md
about: TASK-06306f8cebda
seq: 3
schema: 4
version: 1
---

 checkout ref github.ref (the tag), sh install.sh or powershell -File install.ps1 by OS with every PATH dir holding cargo removed and a check that cargo is gone, './bin/herdr-ank --version' compared to 'herdr-ank $VERSION' -> release.yml; Cargo.toml, Cargo.lock, herdr-plugin.toml at 0.2.0 -> version.rs green; README: Windows in prerequisites and an install paragraph for install.ps1; actionlint clean; no tag set. Nothing outside the criterion. Constraint axis (ADR-599b, ADR-6fb7) re-read on release.yml, Cargo.toml, herdr-plugin.toml, README.md: version refusal unchanged and the two versions agree, five targets unchanged, the smoke runs the two [[build]] scripts only; nothing touches the daemon.
