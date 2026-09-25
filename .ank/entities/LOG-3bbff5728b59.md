---
id: LOG-3bbff5728b59
type: log
title: "review. Against the criterion: release v0.1.0 with four archives + SHA256SUMS (log 1); shell PATH"
created: 2026-09-25T18:56:16Z
author: haksolot@omarchy/ank-1
scope:
  - install.sh
about: TASK-dd28b4ee7cb1
seq: 9
schema: 4
version: 1
---

 without cargo (command -v cargo exit 1) and its propagation to [[build]] measured (log 2); unlink, install --yes succeeded, plugin list shows ank from the managed checkout with bin/herdr-ank, action list answers, uninstall, link restored (logs 3-5). No file of the repository changed: the task scope (install.sh) is untouched. Against constraints (ank context install.sh: ADR-7aab, ADR-6fb7): ADR-7aab's claim that a user without cargo gets the release binary is now measured true on linux x86_64; nothing found.
