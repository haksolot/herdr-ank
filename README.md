# herdr-ank

A [herdr](https://herdr.dev) plugin for [ank](https://github.com/haksolot/ank):
the task each agent holds shows in the sidebar, one action gives a task its own
worktree and agent, `ank tui` opens over the workspace, and the daemon tells you
when something moves.

It is one Rust binary, `herdr-ank`; every entry of `herdr-plugin.toml` runs one
of its subcommands. It reads and writes the corpus only through
`ank <verb> --json`, and drives herdr only through `$HERDR_BIN_PATH`.

## Install

Requires herdr ≥ 0.9.1, `ank` on `PATH`, and a Rust toolchain: herdr builds the
plugin with `cargo build --release` when it installs it. Linux and macOS.

```sh
herdr plugin install haksolot/herdr-ank        # asks before building
herdr plugin install haksolot/herdr-ank --yes  # without the prompt
herdr plugin list --json                       # lists "ank"
```

For development, link a checkout instead. `link` never builds, so build first
and after every change:

```sh
cargo build --release
herdr plugin link .
```

## What it does

| Function | Triggered by |
|---|---|
| **Sidebar tokens.** Each agent pane inside a corpus reports the task it holds, its title, the minutes left on the claim and the claimable count (tokens below). | The daemon, started by herdr at launch (`[[startup]]`): it syncs on herdr events, on `ank watch`'s `events.jsonl` when it exists, and every `sync.poll_seconds`. |
| **Work a task.** A popup lists the claimable tasks of the workspace's corpus, filtered as you type. The one you pick gets a worktree on `task/<short id>` cut from the default branch, a tab with `ANK_AGENT=<user>@<host>/ank-<short id>`, an agent of kind `agent.kind`, and the prompt `ank claim <id>`. If somebody else holds the task, you are notified and the worktree is kept. | The action *Work a task* in the command palette, or `herdr plugin action invoke work --plugin ank`. |
| **ank tui.** `ank tui` over the corpus of the current workspace, as an overlay; `q` closes it. | `herdr plugin pane open --plugin ank --entrypoint tui`. |
| **Notifications.** A task held by an agent pane is done (“TASK-xxxx terminée par ank-xxxx”), a claim has less than `notify.expiring_minutes` left (once per claim), the ratification queue grew (“N décisions en attente”, then `ank review`). | The daemon, between two syncs. |

The worktrees go to `~/.herdr/worktrees/<repository>/ank-<short id>`.

## Sidebar

herdr shows the tokens once your sidebar layout names them. In
`~/.config/herdr/config.toml` (`herdr --help` prints the path on your system):

```toml
[ui.sidebar.agents]
rows = [
  ["state_icon", "machine", "workspace", "tab"],
  ["agent", "$ank_task", "$ank_expires"],
  [{ token = "$ank_title", dim = true }],
]
```

| Token | Value | Absent when |
|---|---|---|
| `ank_task` | the short id of the held task, `TASK-6da1` | no claim is attributed to the pane |
| `ank_title` | its title, cut at 60 characters | no claim is attributed to the pane |
| `ank_expires` | minutes before the claim expires | no claim is attributed to the pane |
| `ank_ambiguous` | `1` when a bare `<user>@<host>` claim shows on several panes | the attribution is unambiguous |
| `ank_queue` | claimable tasks in the corpus | never, inside a corpus |

A claim is attributed to the pane whose herdr agent name ends its identity:
`…/ank-6da1` goes to the agent named `ank-6da1`. Renaming the agent breaks
that link. Every report expires after 90 s, so a stopped daemon empties the
sidebar instead of showing a stale claim.

## Configuration

herdr v1 has no settings UI for plugins. herdr-ank reads `config.toml` in the
directory `herdr plugin config-dir ank` prints, which survives reinstalls.
The file is optional: a missing file or key takes the default below, an unknown
key is ignored, and a key of the wrong type stops the plugin with an error
naming that key.

| Key                       | Type             | Default    | Meaning                                                  |
|---------------------------|------------------|------------|----------------------------------------------------------|
| `agent.kind`              | string           | `"claude"` | herdr agent started for a task                           |
| `agent.args`              | array of strings | `[]`       | extra arguments for that agent; not passed yet           |
| `sync.poll_seconds`       | integer ≥ 0      | `30`       | seconds between two polls of the corpus                  |
| `notify.done`             | boolean          | `true`     | notify when a task is finished                           |
| `notify.expiring_minutes` | integer ≥ 0      | `10`       | notify when a claim has fewer minutes left; `0` turns it off |
| `notify.review`           | boolean          | `true`     | notify when decisions are waiting to be ratified         |

```toml
[agent]
kind = "claude"
args = []

[sync]
poll_seconds = 30

[notify]
done = true
expiring_minutes = 10
review = true
```
