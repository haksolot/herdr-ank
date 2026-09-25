---
id: LOG-aa44dcb549e1
type: log
title: "Windows proof: CI run https://github.com/haksolot/herdr-ank/actions/runs/36182622511 on task/4e8f"
created: 2026-09-25T19:57:41Z
author: haksolot@omarchy/ank-2
scope:
  - install.ps1
  - herdr-plugin.toml
  - .github/workflows/release.yml
  - tests/install_ps1.rs
  - README.md
  - tests/install.rs
about: TASK-4e8fd40b787a
seq: 10
schema: 4
version: 1
---

 @fc3719b: ubuntu, macos, windows all success; windows job runs tests\install_ps1.rs 3/3 ok (valid archive -> bin\herdr-ank.exe, wrong sum -> exit 1 nothing installed, missing archive without cargo -> exit 1 naming the URL and cargo). Before it, run 36181982104: the setup helper's Get-FileHash had failed silently under -Command (exit 0), writing an empty sum; the helper now stops on any error and builds zip and sum with .NET.
