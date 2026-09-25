---
id: LOG-fdd7c45b27fe
type: log
title: "self-review, criterion axis: (1) TUI: src/tui.rs hand_over — cfg(unix) exec, cfg(windows)"
created: 2026-09-25T19:34:52Z
author: haksolot@omarchy/ank-1
scope:
  - src/tui.rs
  - src/main.rs
  - src/ank/**
  - src/work/**
  - tests/**
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
about: TASK-194a3122f31c
seq: 5
schema: 4
version: 1
---

 Command::status then ExitCode of ank's code (none or >255 = 1); tests/tui.rs runs herdr-ank tui against a fake ank on PATH exiting 3 and asserts exit 3 and argv [tui], green on the three OSes. (2) worktree root: src/main.rs const HOME = HOME (cfg unix) / USERPROFILE (cfg windows); no test (it would restate the const); .join(".herdr").join("worktrees") so the path has one separator style. (3) fakes: tests/support/fake_cli.rs, a [[bin]] fake-cli reached by CARGO_BIN_EXE_fake-cli, hard-linked per test (no write handle, so the ETXTBSY retry loops are gone), records argv+ANK_AGENT in calls.jsonl and replays the first matching rule of answers.json; herdr_client, ank_client and work use it; no sh fake of herdr or ank remains; PermissionsExt / std::os::unix outside cfg gone: install.rs is #![cfg(unix)] (install.sh is POSIX sh, per the task body), herdr_client's UnixListener serve is cfg(unix). (4) proof above. Hunks beyond the letter, each needed for a green windows job: work.rs builds context JSON with serde_json (a raw Windows path is invalid JSON); tests move to CARGO_TARGET_TMPDIR so hard links stay on one volume, and serve() keeps its socket in temp_dir to fit sun_path; Cargo.toml default-run = herdr-ank because the package now has two bins. Constraint axis (ank context src/tui.rs, src/main.rs, Cargo.toml, tests/support/fake_cli.rs): ADR-599b — cfg(unix)/cfg(windows) each with its counterpart (tui hand_over, main HOME); tension: 'un seul binaire Rust, herdr-ank' vs the fake-cli [[bin]] the criterion itself names; release archives and ./bin/ still carry herdr-ank only; flagged for planning. ADR-357c, ADR-3cd1, ADR-fa8b: untouched (fake reads/writes only test dirs).
