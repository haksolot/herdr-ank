---
id: LOG-dcd9be3768d4
type: log
title: "discrepancy: criterion assumes 'ank tui --repo <dir>' shows that corpus; measured with ank 0.8.0 in"
created: 2026-09-25T17:34:37Z
author: haksolot@omarchy/ank-3
scope:
  - src/tui.rs
  - herdr-plugin.toml
about: TASK-8e7f97898a70
seq: 7
schema: 4
version: 1
---

 a pty (8 s): 'ank tui' from the repo reads 61 entities, 'ank tui --repo <repo>' reads 0 from any cwd, including the repo itself and with --worktree added. The exec clause and the 'displays the TUI' clause cannot both hold until ank fixes --repo in tui
