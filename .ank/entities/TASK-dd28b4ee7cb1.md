---
id: TASK-dd28b4ee7cb1
type: task
slug: mesure-herdr-plugin-install-r-ussit-sans-cargo-d
title: "Mesure : herdr plugin install réussit sans cargo dans le PATH, depuis la première release"
created: 2026-09-25T18:37:50Z
author: haksolot@omarchy
status: open
scope:
  - install.sh
blocked_by: [TASK-53028f116db2, TASK-a297352636ac]
done_criteria: |
  Une release `v<version>` existe sur github.com/haksolot/herdr-ank avec les quatre archives et SHA256SUMS (posée par l'humain ; sinon la tâche est relâchée en le disant). Mesuré et consigné dans le log : dans un shell dont le PATH ne contient pas cargo, après `herdr plugin unlink ank`, `herdr plugin install haksolot/herdr-ank --yes` réussit, `herdr plugin list --json` liste `ank` avec le binaire sous `bin/` du checkout géré, et `herdr plugin action list --plugin ank` répond ; puis `herdr plugin uninstall ank` et `herdr plugin link /home/haksolot/Projects/herdr-ank` rendent le lien.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
schema: 4
version: 3
---

C'est la question de départ : un utilisateur sans toolchain Rust peut-il
installer le plugin ? Seule une release réelle y répond. herdr lance
[[build]] avec son propre environnement : si le PATH réduit du shell ne se
propage pas au serveur, le noter, et mesurer alors en renommant cargo le temps
du test n'est pas acceptable ; préférer un utilisateur ou un conteneur sans
cargo, ou relâcher en le disant.
