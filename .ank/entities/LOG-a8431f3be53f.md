---
id: LOG-a8431f3be53f
type: log
title: "Tests de bienvenue durcis après un échec vu sous charge : (1) calls.jsonl peut porter une ligne"
created: 2026-09-26T08:47:57Z
author: haksolot@vmi3223161/ank-4fbf
scope:
  - src/notify.rs
  - src/daemon/**
  - tests/notify.rs
  - tests/daemon.rs
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
about: TASK-71c4fcd04466
seq: 2
schema: 4
version: 1
---

 mêlée quand le daemon lance plusieurs faux herdr à la fois (writeln! concurrents du fake, tests/support hors scope) ; le test attend désormais la ligne 'herdr-ank daemon: sync' sur stderr et compte les notifications sur tout le fichier, la bienvenue partant avant tout thread. (2) Un panic pendant l'attente laissait le daemon vivant : garde Running qui le tue au drop ; 6 daemons orphelins tués. (3) tests/daemon.rs créait ses dossiers dans /tmp (tmpfs 3,9 Go) où le faux binaire est copié, 14 Mo par test : /tmp était plein, 709 dossiers morts supprimés ; tempdir passe sous CARGO_TARGET_TMPDIR (hard link). 75 passages sous charge sans échec ; mutation du marqueur toujours tuée.
