---
id: LOG-8a1e3c3eae6b
type: log
title: "review: criterion — [[panes]] id=tui placement=overlay command herdr-ank tui (manifest, as"
created: 2026-09-25T17:23:01Z
author: haksolot@omarchy/ank-3
scope:
  - src/tui.rs
  - herdr-plugin.toml
about: TASK-8e7f97898a70
seq: 4
schema: 4
version: 1
---

 target/release/herdr-ank per ADR-aca6 since pane cwd is the plugin root); reads HERDR_PLUGIN_CONTEXT_JSON, worktree.checkout_path else workspace_cwd (start_dir + 3 tests); walks up to first dir with .ank/ dir (find_corpus + 2 tests, mutation-checked); exec ank tui --repo (CommandExt::exec, checked with fake ank on PATH); not found -> stderr path + 'ank init', exit 1 (test + binary run). Unanswered: live 'herdr plugin pane open' on ank.tui. constraints — ADR-aca6 over herdr-plugin.toml: command is the release binary, no script; ADR-357c/3cd1/fa8b: no herdr call, no .ank read (only is_dir on the path to locate the repo, not opening files under it), no agent start; nothing found
