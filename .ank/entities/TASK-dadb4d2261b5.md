---
id: TASK-dadb4d2261b5
type: task
slug: client-ank-ank-verbe-json-typ-contrat-v-rifi-cod
title: "Client ank : ank <verbe> --json typé, contrat vérifié, codes de sortie routés"
created: 2026-09-25T16:54:18Z
author: haksolot@omarchy
status: open
scope:
  - src/ank/**
  - tests/ank_client.rs
  - tests/fixtures/ank/**
blocked_by: [TASK-5dbc8dd1aebf]
done_criteria: |
  Un module `ank` expose `Client::new(repo: &Path)` et une méthode par verbe utilisé (`status`, `find`, `show`, `context`) qui exécute `ank <verbe> --json --repo <repo>` et rend `Result<T, AnkError>`. `AnkError` porte le code de sortie 1 à 9 et le message stderr ; un code 3 est distingué de 4, 6 de 7, 9 de tout le reste. Un document dont `contract` n'est pas 1 est refusé avec une erreur nommée. Les tests désérialisent chaque fixture copiée de `crates/ank-cli/tests/golden-json/` du dépôt ank pour les verbes utilisés, et un champ inconnu ajouté à une fixture ne fait échouer aucun test.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
schema: 4
version: 1
---

La frontière avec ank (ADR-3cd19cd6acb9). Tout ce que le plugin sait d'un corpus passe
ici, et rien d'autre dans le crate ne spawne `ank`.

Points à tenir :
- `ank help --json` publie les champs de chaque document et lequel est
  `nullable` ; `branch` et `default_branch` de `status` le sont (HEAD détaché,
  corpus sans remote).
- `find --status in_progress --json` porte `coordination` par résultat, c'est
  la source du mapping claim → identité.
- `context --json` porte les tâches claimables : source de `ank_queue`.
- les fixtures golden-json sont capturées depuis le processus : les copier
  telles quelles, avec un README qui dit d'où et à quel commit.
