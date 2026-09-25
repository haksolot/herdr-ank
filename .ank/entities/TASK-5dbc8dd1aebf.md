---
id: TASK-5dbc8dd1aebf
type: task
slug: squelette-crate-herdr-ank-et-herdr-plugin-toml-q
title: "Squelette : crate herdr-ank et herdr-plugin.toml que herdr plugin link accepte"
created: 2026-09-25T16:54:17Z
author: haksolot@omarchy
status: open
scope:
  - Cargo.toml
  - herdr-plugin.toml
  - src/main.rs
  - .gitignore
blocked_by: []
done_criteria: |
  `cargo build --release` produit `target/release/herdr-ank` ; `herdr-ank --version` imprime la version du Cargo.toml. `herdr-plugin.toml` à la racine déclare id `ank`, name, version, min_herdr_version `0.9.1`, platforms `["linux","macos"]`, un `[[build]]` `cargo build --release` ; `herdr plugin link .` réussit et `herdr plugin list --json` liste `ank` avec `enabled: true`.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
schema: 4
version: 1
---

Première brique, sans fonction : le plugin existe pour herdr et le binaire
existe pour le plugin. Tout le reste s'y accroche (ADR-aca6aaeb3a5f).

Le manifeste ne déclare encore ni action ni pane ni startup : chaque tâche
en aval ajoute la sienne. Le binaire est un `clap` avec une sous-commande par
entrée future (`daemon`, `sync`, `work`, `tui`) qui répondent « pas encore
implémenté » avec code 1, pour que le manifeste puisse être complété avant
que chaque fonction ne le soit.

Édition Rust 2021, MSRV celle de la toolchain stable courante. Un seul crate,
pas de workspace tant qu'un second binaire n'existe pas.
