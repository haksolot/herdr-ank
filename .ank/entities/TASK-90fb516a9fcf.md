---
id: TASK-90fb516a9fcf
type: task
slug: configuration-du-plugin-dans-herdr-plugin-config
title: Configuration du plugin dans $HERDR_PLUGIN_CONFIG_DIR/config.toml avec défauts documentés
created: 2026-09-25T16:54:22Z
author: haksolot@omarchy
status: done
scope:
  - src/config.rs
  - tests/config.rs
  - README.md
blocked_by: [TASK-5dbc8dd1aebf]
done_criteria: |
  `Config::load(dir)` lit `config.toml` dans le répertoire donné et rend les défauts quand le fichier manque : `agent.kind = "claude"`, `agent.args = []`, `sync.poll_seconds = 30`, `notify.done = true`, `notify.expiring_minutes = 10`, `notify.review = true`. Une clé inconnue est ignorée, une valeur du mauvais type est une erreur nommant la clé. Les tests couvrent fichier absent, fichier partiel, et clé mal typée. Le README liste chaque clé et son défaut.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
proof:
  - type: test
    ref: local/b89bba2843c9@97899fd
    tree: scope/7bf2720524ae
    criteria: 101d89bf712d
    verifier: cargo-test@f14aeab36e1b
    via: verifier
  - type: test
    ref: local/e3b0c44298fc@97899fd
    tree: scope/7bf2720524ae
    criteria: 101d89bf712d
    verifier: clippy@d335c02ef52c
    via: verifier
  - type: test
    ref: local/e3b0c44298fc@97899fd
    tree: scope/7bf2720524ae
    criteria: 101d89bf712d
    verifier: fmt-check@5ca6d10bcd55
    via: verifier
schema: 4
version: 3
---

Le plugin n'a pas d'UI de réglage : herdr v1 ne l'offre pas. Le fichier vit
dans le répertoire que `herdr plugin config-dir ank` imprime, persistant
entre réinstallations.

Le `kind` par défaut vaut `claude` parce que c'est l'intégration herdr
installée sur la machine de départ ; il reste une préférence et non une
décision.
