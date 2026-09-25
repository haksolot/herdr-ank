---
id: LOG-0433f1bdd26e
type: log
title: herdr appends .exe itself, so no platforms=[windows] variants are needed (herdrdev/herdr @21d0ce6).
created: 2026-09-25T19:13:11Z
author: haksolot@omarchy/ank-2
scope:
  - install.ps1
  - herdr-plugin.toml
  - .github/workflows/release.yml
  - tests/install_ps1.rs
  - README.md
about: TASK-4e8fd40b787a
seq: 2
schema: 4
version: 1
---

 Actions/startup/events/build: src/plugin_command.rs command_for_argv_in_dir -> program_for_cwd joins './bin/herdr-ank' onto the plugin root, resolve_windows_program returns None for a path with a separator, so std::process::Command::new(<root>\bin\herdr-ank) spawns it, and Rust std's resolve_exe appends EXE_SUFFIX to any non-bare path that lacks it and exists with it. Panes: src/app/api/plugins/mod.rs:485 rewrites command[0] with program_for_cwd, portable-pty (vendor) CommandBuilder::search_path on Windows tries PATH.join(exe) then with_extension(PATHEXT entry), and join of an absolute path is that path, so <root>\bin\herdr-ank.EXE is found. [[build]] takes its own platforms (src/app/api/plugins/manifest.rs:261, cli/plugin.rs:1241 build_platform_supported).
