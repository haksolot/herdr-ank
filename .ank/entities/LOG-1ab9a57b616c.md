---
id: LOG-1ab9a57b616c
type: log
title: "review: criterion — [[panes]] tui overlay command herdr-ank tui (manifest); reads"
created: 2026-09-25T17:45:48Z
author: haksolot@omarchy/ank-3
scope:
  - src/tui.rs
  - herdr-plugin.toml
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-8e7f97898a70
seq: 12
schema: 4
version: 1
---

 HERDR_PLUGIN_CONTEXT_JSON, worktree.checkout_path else workspace_cwd (3 tests); walks up to first .ank/ dir (2 tests, mutation-checked); exec 'ank tui' with that dir as cwd, no --repo (ank_tui test red then green, fake-ank run shows args 'tui' and the cwd); not found: stderr path + ank init, exit 1 (test + binary run); live pane shows the corpus (log above). Nothing beyond it: main.rs/lib.rs/manifest lines are registration only. constraints — ADR-aca6: manifest command is ./target/release/herdr-ank, no script; ADR-3cd1: nothing under .ank/ opened (is_dir on the path only), the TUI is ank's own; ADR-357c, ADR-fa8b: no herdr call, no agent start; nothing found
