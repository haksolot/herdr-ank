This repo uses Ank: tasks and decisions live in `.ank/`.

## Working in this repository

- One agent, one worktree, one identity. `ANK_AGENT` is set in the pane herdr
  opened for you. `/home/haksolot/Projects/herdr-ank` is the integration tree on
  `main`: never edit a file there.
- A task is one branch cut from `main` at claim time:
  `git checkout -b task/<short id> main`. There is no remote; nothing to fetch.
- Landing, after `ank done` and a commit of everything the task touched
  (including `.ank/entities`):

      git rebase main
      git -C /home/haksolot/Projects/herdr-ank merge --ff-only task/<short id>

  If the fast-forward is refused, `main` moved: rebase again and retry. Never
  `--no-ff`, never resolve a conflict in the integration tree. The next pass
  starts from a fresh branch off the new `main`.
- Verifiers are `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`;
  `ank done` runs them itself. `target/` is ignored by git.
- Commit messages: one line on what and why; no attribution trailer needed.
