---
id: LOG-eb19ca02c8ef
type: log
title: "review, axis criterion: plan(panes, agents, corpora, config) -> Vec<Report> pure (no I/O, inputs"
created: 2026-09-25T17:34:12Z
author: haksolot@omarchy/ank-2
scope:
  - src/sync/**
  - tests/sync.rs
about: TASK-416cdde27bfb
seq: 8
schema: 4
version: 1
---

 only); Report carries pane_id, source ank:sync, the five tokens in spec order, the clears, ttl 90000. Tests: suffix /<nom> to that agent's pane only, and not to a same-named agent in another worktree; fallback identity on two panes of one worktree -> ank_ambiguous=1, single pane -> not, and ambiguity counts only panes the claim is actually shown on (a red found a first draft counting every agent pane); no agent / no corpus / sibling path -> nothing; vanished claim in a live corpus clears four and keeps ank_queue; claim gone with its corpus clears all five; deepest root wins; title truncated to 60 chars. Unasked hunks: ttl derived from config, removed. Wiring outside scope: 'pub mod sync;' in src/lib.rs, same one line as ank/config/herdr
