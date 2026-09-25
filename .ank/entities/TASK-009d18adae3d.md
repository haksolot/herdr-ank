---
id: TASK-009d18adae3d
type: task
slug: client-herdr-d-coder-les-v-nements-que-le-daemon
title: "Client herdr : décoder les événements que le daemon doit recevoir (pane.agent_detected, worktree.*, workspace.closed)"
created: 2026-09-25T17:45:15Z
author: haksolot@omarchy/ank-2
status: done
scope:
  - src/herdr/**
  - tests/herdr_client.rs
  - tests/fixtures/herdr/**
blocked_by: [TASK-c5be7ea3d8c7]
done_criteria: |
  Le flux de `Client::subscribe` rend un `Event` pour chaque type d'événement listé par ADR-c8e7e56e5219 : en plus de pane.created, pane.closed et pane.agent_status_changed déjà décodés, pane.agent_detected, worktree.created, worktree.opened, worktree.removed et workspace.closed, sous leur forme d'enveloppe herdr 0.9.1 (`pane_agent_detected`, `worktree_created`…). Un type hors de cette liste reste ignoré. Le test de décodage lit une fixture NDJSON contenant chacun de ces types et un type inconnu.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
proof:
  - type: test
    ref: local/f1cc51f89c86@24cbdef
    tree: scope/ecf0b6144654
    criteria: b3f33d9c53ce
    verifier: cargo-test@f14aeab36e1b
    via: verifier
  - type: test
    ref: local/e3b0c44298fc@24cbdef
    tree: scope/ecf0b6144654
    criteria: b3f33d9c53ce
    verifier: clippy@d335c02ef52c
    via: verifier
  - type: test
    ref: local/e3b0c44298fc@24cbdef
    tree: scope/ecf0b6144654
    criteria: b3f33d9c53ce
    verifier: fmt-check@5ca6d10bcd55
    via: verifier
schema: 4
version: 3
---

Trouvé en ouvrant TASK-91a1 : ADR-c8e7 veut une synchronisation à chaque événement reçu de ces types, mais TASK-de0a n'a modélisé que les événements de pane et le flux ignore le reste, si bien que le daemon ne serait jamais réveillé par un worktree créé ou un workspace fermé. Formes mesurées dans `herdr api schema --json` (schemas.event.$defs.EventData) : worktree_* portent workspace et worktree (WorktreeInfo), worktree_removed porte workspace_id, workspace_closed porte workspace_id, pane_agent_detected porte pane_id, workspace_id, agent.
