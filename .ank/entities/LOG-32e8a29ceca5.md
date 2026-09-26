---
id: LOG-32e8a29ceca5
type: log
title: "Real herdr 0.9.1 session measured, on a COPY: isolated session ank-measure (own socket"
created: 2026-09-26T23:21:27Z
author: haksolot@vmi3223161/ank-0b72
scope:
  - src/setup.rs
  - tests/setup.rs
  - README.md
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-0b72dd2eb921
seq: 2
schema: 4
version: 1
---

 ~/.config/herdr/sessions/ank-measure/herdr.sock) hosted in tmux, HERDR_CONFIG_PATH = copy of the user's config.toml with its ank tail (sidebars + prefix+a) stripped; real config.toml sha256 8042400b… unchanged before/after, real server never reloaded. Before setup: ctrl+b a opens nothing. herdr-ank setup inside that session: added [[keys.command]], [ui.sidebar.agents], [ui.sidebar.spaces], backup config.toml.bak-ank-20260926232107, real herdr config check ok, reload-config to the isolated server ok. Then ctrl+b a opens the 'Ank' overlay running ank tui over the worktree's corpus (TASK-0b72 listed in_progress); q closes it. Also: setup on an unmodified copy of the user's config (already configured) prints 'nothing to add', file byte-identical, no backup. Session stopped and deleted.
