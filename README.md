# herdr-ank

A [herdr](https://herdr.dev) plugin for [ank](https://github.com/haksolot/ank):
the task each agent holds shows in the sidebar, one action gives a task its own
worktree and agent, `ank tui` opens over the workspace, and the daemon tells you
when something moves.

It is one Rust binary, `herdr-ank`; every entry of `herdr-plugin.toml` runs one
of its subcommands. It reads and writes the corpus only through
`ank <verb> --json`, and drives herdr only through `$HERDR_BIN_PATH`.

## Install

Requires herdr ≥ 0.9.1 and `ank` on `PATH`. Linux and macOS, x86_64 and
aarch64; Windows, x86_64, with Windows PowerShell 5.1 (what Windows ships).

```sh
herdr plugin install haksolot/herdr-ank        # asks before running the install script
herdr plugin install haksolot/herdr-ank --yes  # without the prompt
herdr plugin list --json                       # lists "ank"
```

On Linux and macOS herdr runs the manifest's `[[build]]`, `sh install.sh`, in
the plugin's directory. The script reads `version` from `herdr-plugin.toml`,
picks the archive of the platform `uname -sm` names, and downloads it with its
sums from the GitHub release `v<version>`:

```
https://github.com/haksolot/herdr-ank/releases/download/v<version>/herdr-ank-<version>-<target>.tar.gz
https://github.com/haksolot/herdr-ank/releases/download/v<version>/SHA256SUMS
```

`<target>` is one of `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`,
`x86_64-apple-darwin`, `aarch64-apple-darwin`. The archive is checked against
`SHA256SUMS` (`sha256sum` or `shasum -a 256`), and `herdr-ank` lands in
`./bin/herdr-ank`, which every other entry of the manifest runs. It needs `curl`
or `wget`, `tar` and `gzip`; no Rust toolchain.

cargo only serves when that fails: no release for this version, a platform
without an archive, a download or a sum that fails. Then, if `cargo` is on
`PATH`, the script runs `cargo build --release` and copies
`target/release/herdr-ank` into `./bin/`; without cargo it exits 1, naming the
URL it tried (or the platform) and the missing cargo.
`HERDR_ANK_RELEASE_BASE` replaces the release URL, `file://` included.

On Windows the `[[build]]` is `powershell -NoProfile -ExecutionPolicy Bypass
-File install.ps1`, under the same contract: it downloads
`herdr-ank-<version>-x86_64-pc-windows-msvc.zip` and `SHA256SUMS` from the same
release, checks the sum with `Get-FileHash`, and puts `herdr-ank.exe` in
`.\bin\`, which herdr runs for every `./bin/herdr-ank` of the manifest. Without
the archive it falls back to `cargo build --release` when cargo is present, and
otherwise exits 1 naming the URL and the missing cargo.
`HERDR_ANK_RELEASE_BASE` may also name a local directory holding the archive
and its sums.

For development, link a checkout instead. `link` never builds, and
`install.sh` would fetch the released binary rather than your changes, so build
and copy first and after every change:

```sh
cargo build --release && mkdir -p bin && cp target/release/herdr-ank bin/
herdr plugin link .
```

## What it does

| Function | Triggered by |
|---|---|
| **Sidebar tokens.** Each agent pane inside a corpus reports the task it holds, its title, the minutes left on the claim and the claimable count (tokens below). | The daemon (see *Daemon lifecycle*): it syncs on herdr events, on `ank watch`'s `events.jsonl` when it exists, and every `sync.poll_seconds`. |
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

## Daemon lifecycle

One `herdr-ank daemon` runs per herdr server, holding a lock in the plugin's
state directory. herdr has no supervisor for plugin processes, so the manifest
is the supervisor:

- `[[startup]]` launches it when herdr restores a session;
- `[[events]]` launch it again on `workspace.created`, `tab.created`,
  `pane.agent_detected` and `worktree.opened`. While a daemon holds the lock,
  each of these launches exits 0 at once and writes nothing.

So after `herdr plugin link`, the daemon starts with the first of those
events (opening a tab is enough), and if it dies, the next one brings it
back. Between the two, the sidebar tokens expire on their own after 90 s;
nothing else depends on the daemon being alive. `pgrep -af 'herdr-ank daemon'`
shows it. `herdr plugin log list --plugin ank` lists the live daemon as
`running` and each hook that found it as `succeeded`; herdr shows a
command's stderr, one line per sync, once that command has ended.

## Configuration

herdr v1 has no settings UI for plugins. herdr-ank reads `config.toml` in the
directory `herdr plugin config-dir ank` prints, which survives reinstalls.
The file is optional: a missing file or key takes the default below, an unknown
key is ignored, and a key of the wrong type or out of its range stops the
plugin with an error naming that key.

| Key                       | Type             | Default    | Meaning                                                  |
|---------------------------|------------------|------------|----------------------------------------------------------|
| `agent.kind`              | string           | `"claude"` | herdr agent started for a task                           |
| `agent.args`              | array of strings | `[]`       | extra arguments passed to that agent, after `--`         |
| `sync.poll_seconds`       | integer 1 to 30  | `30`       | seconds between two polls of the corpus                  |
| `notify.done`             | boolean          | `true`     | notify when a task is finished                           |
| `notify.expiring_minutes` | integer ≥ 0      | `10`       | notify when a claim has fewer minutes left; `0` turns it off |
| `notify.review`           | boolean          | `true`     | notify when decisions are waiting to be ratified         |

`sync.poll_seconds` cannot exceed 30. The synchronisation ADR
(ADR-6fb76f3a1197) requires a sync at least every 30 seconds, and the sidebar
tokens are reported with a 90-second ttl (SPEC-dbe3cf972f71), three times that
maximum: a slower poll would let the tokens expire between two syncs and empty
the sidebar.

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
