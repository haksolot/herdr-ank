---
id: ADR-7aab39e2dbd8
type: adr
slug: le-plugin-se-distribue-en-binaire-pr-compil-par
title: Le plugin se distribue en binaire précompilé par release GitHub ; [[build]] est un script d'amorçage, cargo en repli
created: 2026-09-25T18:37:48Z
author: haksolot@omarchy
status: superseded
scope:
  - herdr-plugin.toml
  - install.sh
  - .github/**
  - src/main.rs
constraint: |
  Le plugin reste un seul binaire Rust, herdr-ank, et toute entrée du manifeste autre que [[build]] (startup, actions, panes, events, link_handlers) invoque ./bin/herdr-ank ou ank tui à travers lui. [[build]] est l'unique script du manifeste : install.sh, POSIX sh, qui lit la version du manifeste, télécharge depuis la release GitHub v<version> l'archive de sa plateforme (linux et macos, x86_64 et aarch64, linux en musl), vérifie sa somme dans SHA256SUMS, pose le binaire dans ./bin/herdr-ank, et ne se rabat sur cargo build --release que si le téléchargement ou la vérification échoue et que cargo est présent ; sinon il sort 1 en nommant ce qui manque. Les binaires sont produits par un workflow GitHub Actions déclenché par le tag v<version>, qui refuse un tag dont la version diffère de Cargo.toml ou du manifeste. Windows n'est pas déclaré tant que le socket nommé n'est pas couvert.
supersedes: ADR-aca6aaeb3a5f
ratified: 0bc9ddbd41d5
verified:
  - by: haksolot@omarchy
    at: 2026-09-25T18:37:50Z
schema: 4
version: 3
---

Remplace ADR-aca6aaeb3a5f sur la distribution ; le reste de sa décision tient : un
binaire, une sous-commande par entrée du manifeste.

Le 2026-09-25, l'installation depuis GitHub a été vérifiée sur une seule
machine, qui avait déjà une toolchain Rust. Un utilisateur de herdr n'en a pas
forcément, et `herdr plugin install` abandonne dès que `cargo build` manque :
herdr ne livre pas de binaire, il exécute ce que `[[build]]` déclare. Le plugin
s'adresse à des utilisateurs de herdr, pas à des développeurs Rust.

Trois options pesées :

- garder `cargo build` : simple, mais exige cargo chez chaque utilisateur ;
- commiter les binaires dans le dépôt : pas de CI, mais un dépôt qui grossit
  à chaque version et des binaires que personne ne peut relier à une source ;
- **releases GitHub et script d'amorçage** (retenu) : un workflow produit les
  quatre binaires sur tag, `install.sh` télécharge celui de la plateforme et
  vérifie sa somme, cargo reste un repli pour une plateforme sans asset ou
  une version sans release. `herdr plugin link` ne construit jamais, donc le
  développement ne change pas.

Un binaire téléchargé est vérifié par somme, pas signé : c'est le standard
des plugins herdr, qui ne sont pas sandboxés. Le tag, `Cargo.toml` et le
manifeste portent un seul numéro, et le workflow refuse qu'ils divergent :
un utilisateur qui installe la version N reçoit le binaire construit de N.
