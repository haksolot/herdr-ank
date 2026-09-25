---
id: TASK-b5a0d70fde4b
type: task
slug: client-herdr-plugin-pane-open
title: "Client herdr : plugin pane open"
created: 2026-09-25T17:32:01Z
author: haksolot@omarchy/ank-1
status: done
scope:
  - src/herdr/**
  - tests/herdr_client.rs
  - tests/fixtures/herdr/**
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
blocked_by: []
done_criteria: |
  Le `Client` du module `herdr` expose `plugin_pane_open(entrypoint, workspace, env)` qui exécute `herdr plugin pane open --plugin ank --entrypoint <id> [--workspace <id>] [--env KEY=VALUE]... --focus` sans shell et rend le `plugin_pane` de la réponse `plugin_pane_opened` (au moins `entrypoint` et le `pane_id` de `pane`). Le test vérifie l argv exact sur le binaire factice et le décodage d une réponse `{"id":..,"result":{"type":"plugin_pane_opened","plugin_pane":{...}}}` portant un champ inconnu ; une réponse `error` rend une `HerdrError::Api`.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
proof:
  - type: test
    ref: local/a192eab7375a@fb4705e
    tree: scope/e082003fef02
    criteria: 015449d9c2bf
    verifier: cargo-test@f14aeab36e1b
    via: verifier
  - type: test
    ref: local/e3b0c44298fc@fb4705e
    tree: scope/e082003fef02
    criteria: 015449d9c2bf
    verifier: clippy@d335c02ef52c
    via: verifier
  - type: test
    ref: local/e3b0c44298fc@fb4705e
    tree: scope/e082003fef02
    criteria: 015449d9c2bf
    verifier: fmt-check@5ca6d10bcd55
    via: verifier
schema: 4
version: 3
---

Découvert en préparant TASK-c81a : `herdr-ank work` doit ouvrir la pane `pick`, et le client herdr livré par TASK-de0a ne couvre pas `plugin pane open` (ADR-357c : tout passe par $HERDR_BIN_PATH). Forme mesurée dans `herdr api schema --json` (herdr 0.9.1) : PluginPaneOpenParams {plugin_id, entrypoint, workspace_id?, env{}, focus, placement?, cwd?}, réponse `plugin_pane_opened` {plugin_pane: {plugin_id, entrypoint, pane: PaneInfo}}. Le placement vient du manifeste ; `--placement` de la CLI n accepte pas `popup`, donc ne pas le passer.
