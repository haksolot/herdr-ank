# herdr-ank

## Configuration

herdr v1 has no settings UI for plugins. herdr-ank reads `config.toml` in the
directory `herdr plugin config-dir ank` prints, which survives reinstalls.
The file is optional: a missing file or key takes the default below, an unknown
key is ignored, and a key of the wrong type stops the plugin with an error
naming that key.

| Key                       | Type             | Default    | Meaning                                                  |
|---------------------------|------------------|------------|----------------------------------------------------------|
| `agent.kind`              | string           | `"claude"` | herdr agent started for a task                           |
| `agent.args`              | array of strings | `[]`       | extra arguments passed to that agent                     |
| `sync.poll_seconds`       | integer ≥ 0      | `30`       | seconds between two polls of the corpus                  |
| `notify.done`             | boolean          | `true`     | notify when a task is finished                           |
| `notify.expiring_minutes` | integer ≥ 0      | `10`       | notify when a claim has this many minutes left           |
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
