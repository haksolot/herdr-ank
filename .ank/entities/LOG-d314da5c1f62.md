---
id: LOG-d314da5c1f62
type: log
title: "the installed binary is the release's, not a build: managed checkout"
created: 2026-09-25T18:56:14Z
author: haksolot@omarchy/ank-1
scope:
  - install.sh
about: TASK-dd28b4ee7cb1
seq: 8
schema: 4
version: 1
---

 /home/haksolot/.config/herdr/plugins/github/ank-30804615d55e has bin/herdr-ank (static-pie, 2712840 bytes) and no target/; its sha256 caa64b04ea93f8e6155b92333ef6bd2f994306b6239aba5861bdf4a8c16753c5 equals herdr-ank inside herdr-ank-0.1.0-x86_64-unknown-linux-musl.tar.gz from gh release download. Link restored to /home/haksolot/Projects/herdr-ank (source local, enabled). Left as found: the daemon already running before this pass (pid 171745) still runs /home/haksolot/Projects/herdr-ank/target/release/herdr-ank from before TASK-5302; the manifest's next event relaunch uses ./bin/herdr-ank, which exists in the integration tree.
