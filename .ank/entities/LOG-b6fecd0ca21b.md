---
id: LOG-b6fecd0ca21b
type: log
title: "self-review: criterion axis — ci.yml triggers push branches '**' + pull_request, matrix"
created: 2026-09-25T19:12:10Z
author: haksolot@omarchy/ank-1
scope:
  - .github/workflows/ci.yml
  - AGENTS.md
  - README.md
about: TASK-678211d8548c
seq: 3
schema: 4
version: 1
---

 ubuntu/macos/windows, fmt --check, clippy --workspace --all-targets -D warnings, test --workspace --locked, job name = matrix.os; actionlint 1.7.12 -shellcheck= reports nothing (ci.yml and release.yml); AGENTS.md: push task/<id>, gh run watch, delete remote branch after landing, never push main/tag; also removed the stale 'There is no remote' line it contradicted. README.md in scope but no clause asks for it: untouched. Constraint axis — ank context .github/workflows/ci.yml AGENTS.md: ADR-599b binds .github/**: CI is the measuring machine, release workflow untouched. Nothing found.
