---
id: LOG-8b90aae8f81b
type: log
title: "measurement, final run, verbatim (PATH=/usr/bin):"
created: 2026-09-25T18:56:12Z
author: haksolot@omarchy/ank-1
scope:
  - install.sh
about: TASK-dd28b4ee7cb1
seq: 7
schema: 4
version: 1
---


$ command -v cargo
exit=1
$ herdr plugin list --json   (before)
{"id":"cli:plugin","result":{"plugins":[],"type":"plugin_list"}}
exit=0
$ herdr plugin install haksolot/herdr-ank --yes
Plugin install preview:
  id: ank
  name: Ank
  version: 0.1.0
  source: haksolot/herdr-ank
  commit: eb0433e090519f2348a3f26f38b24b27b92be091
  actions: 1
  startup commands: 1
  events: 4
  panes: 2
  link handlers: 0
  build commands: 1
    build: sh install.sh
Installed ank from haksolot/herdr-ank.
Config: /home/haksolot/.config/herdr/plugins/config/ank
exit=0
$ herdr plugin list --json
{'plugin_id': 'ank', 'plugin_root': '/home/haksolot/.config/herdr/plugins/github/ank-30804615d55e', 'source': {'installed_unix_ms': 1790362547647, 'kind': 'github', 'managed_path': '/home/haksolot/.config/herdr/plugins/github/ank-30804615d55e', 'owner': 'haksolot', 'repo': 'herdr-ank', 'resolved_commit': 'eb0433e090519f2348a3f26f38b24b27b92be091'}}
exit=0
$ ls -d $root/target; sha256sum $root/bin/herdr-ank
-rwxr-xr-x 1 haksolot haksolot 2712840 Sep 25 20:55 /home/haksolot/.config/herdr/plugins/github/ank-30804615d55e/bin/herdr-ank
ls: cannot access '/home/haksolot/.config/herdr/plugins/github/ank-30804615d55e/target': No such file or directory
caa64b04ea93f8e6155b92333ef6bd2f994306b6239aba5861bdf4a8c16753c5  /home/haksolot/.config/herdr/plugins/github/ank-30804615d55e/bin/herdr-ank
$ herdr plugin action list --plugin ank
{"id":"cli:plugin","result":{"actions":[{"action_id":"work","command":["./bin/herdr-ank","work"],"contexts":["workspace"],"platforms":["linux","macos"],"plugin_id":"ank","title":"Work a task"}],"type":"plugin_action_list"}}
exit=0
$ herdr plugin uninstall ank
Uninstalled ank.
exit=0
$ herdr plugin link /home/haksolot/Projects/herdr-ank
{"id":"cli:plugin","result":{"plugin":{"actions":[{"command":["./bin/herdr-ank","work"],"contexts":["workspace"],"id":"work","title":"Work a task"}],"build":[{"command":["sh","install.sh"]}],"description":"Ank tasks, claims and decisions in herdr","enabled":true,"events":[{"command":["./bin/herdr-ank","daemon"],"on":"pane.agent_detected"},{"command":["./bin/herdr-ank","daemon"],"on":"tab.created"},{"command":["./bin/herdr-ank","daemon"],"on":"workspace.created"},{"command":["./bin/herdr-ank","daemon"],"on":"worktree.opened"}],"manifest_path":"/home/haksolot/Projects/herdr-ank/herdr-plugin.toml","min_herdr_version":"0.9.1","name":"Ank","panes":[{"command":["./bin/herdr-ank","pick"],"id":"pick","placement":"popup","title":"Pick a task"},{"command":["./bin/herdr-ank","tui"],"id":"tui","placement":"overlay","title":"Ank"}],"platforms":["linux","macos"],"plugin_id":"ank","plugin_root":"/home/haksolot/Projects/herdr-ank","source":{"kind":"local"},"startup":[{"command":["./bin/herdr-ank","daemon"]}],"version":"0.1.0"},"type":"plugin_linked"}}
exit=0
$ herdr plugin list --json   (after)
{'plugin_id': 'ank', 'plugin_root': '/home/haksolot/Projects/herdr-ank', 'source': {'kind': 'local'}, 'enabled': True}
