---
id: LOG-a3cb4e3d72e5
type: log
title: "discrepancy: the criterion says curl or wget; wget is not installed here and does not read file://"
created: 2026-09-25T18:41:30Z
author: haksolot@omarchy/ank-1
scope:
  - install.sh
  - herdr-plugin.toml
  - tests/install.rs
  - README.md
  - .gitignore
about: TASK-53028f116db2
seq: 3
schema: 4
version: 1
---

 URLs, so only the curl branch is exercised by tests/install.rs; likewise only sha256sum, not shasum -a 256. POSIX: no dash/shellcheck/busybox on this machine, checked with bash --posix -n only.
