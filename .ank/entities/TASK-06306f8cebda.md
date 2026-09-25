---
id: TASK-06306f8cebda
type: task
slug: le-workflow-de-release-installe-le-plugin-depuis
title: Le workflow de release installe le plugin depuis la release fraîche sur les trois OS, et la version passe à 0.2.0
created: 2026-09-25T19:06:40Z
author: haksolot@omarchy
status: done
scope:
  - .github/workflows/release.yml
  - Cargo.toml
  - Cargo.lock
  - herdr-plugin.toml
  - README.md
blocked_by: [TASK-194a3122f31c, TASK-4e8fd40b787a]
done_criteria: |
  `release.yml` gagne un job `smoke` après `release`, matrice ubuntu-latest, macos-latest, windows-latest, qui fait un checkout du tag, lance `sh install.sh` ou `install.ps1` selon l'OS avec un PATH sans cargo (`rustup` absent du runner ou PATH réduit), et vérifie que `./bin/herdr-ank --version` imprime la version du tag. `Cargo.toml` et `herdr-plugin.toml` passent à 0.2.0 et `tests/version.rs` reste vert. Le README dit Windows dans les prérequis et l'installation. actionlint ne rapporte rien. Aucun tag n'est posé par cette tâche.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
proof:
  - type: test
    ref: local/b01cc51e97ef@25ff57c
    tree: scope/021bf134ae5a
    criteria: 1f3aaf180c3e
    verifier: cargo-test@f14aeab36e1b
    via: verifier
  - type: test
    ref: local/e3b0c44298fc@25ff57c
    tree: scope/021bf134ae5a
    criteria: 1f3aaf180c3e
    verifier: clippy@d335c02ef52c
    via: verifier
  - type: test
    ref: local/e3b0c44298fc@25ff57c
    tree: scope/021bf134ae5a
    criteria: 1f3aaf180c3e
    verifier: fmt-check@5ca6d10bcd55
    via: verifier
schema: 4
version: 3
---

Dernière tâche avant la release v0.2.0, que l'humain pose. Le job smoke
est la mesure « installation sans cargo » de TASK-dd28 rejouée à chaque
release, sur les trois OS, sans machine à disposition.
