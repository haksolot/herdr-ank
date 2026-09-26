---
id: TASK-0b72dd2eb921
type: task
slug: herdr-ank-setup-ajoute-une-fois-au-config-toml-d
title: "herdr-ank setup : ajoute une fois au config.toml de herdr le raccourci prefix+a d'Ouvrir ank et les lignes de sidebar, et le README cesse de citer une palette que herdr 0.9.1 n'a pas"
created: 2026-09-26T23:12:00Z
author: haksolot@vmi3223161/ank-plan
status: in_progress
scope:
  - src/setup.rs
  - tests/setup.rs
  - README.md
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/main.rs
blocked_by: []
done_criteria: |
  herdr-ank setup ajoute au config.toml de herdr (le chemin que herdr documente, ~/.config/herdr/config.toml sur Linux et macOS, surchargeable pour les tests) le bloc [[keys.command]] prefix+a type plugin_action command ank.open et les blocs [ui.sidebar.agents] et [ui.sidebar.spaces] du README, après une sauvegarde config.toml.bak-ank-<horodatage>, puis lance herdr config check et herdr server reload-config par $HERDR_BIN_PATH (ADR-357c). Idempotent : relancé, il n'écrit rien et ne crée aucune sauvegarde. Il n'écrase jamais une clé déjà définie par l'utilisateur : si prefix+a est déjà lié à autre chose, ou si [ui.sidebar.agents] ou [ui.sidebar.spaces] existe déjà, il laisse ce bloc intact et le nomme sur stderr avec la ligne à ajouter à la main ; si herdr config check signale une erreur, il restaure la sauvegarde et sort 1. tests/setup.rs le vérifie sur un config.toml temporaire et le binaire herdr factice : config vierge, config déjà configurée (aucune écriture), conflit prefix+a, sidebar existante, check en échec. Le README ne parle plus de palette de commandes (herdr 0.9.1 n'en a pas) : il nomme herdr-ank setup à l'installation, prefix+a (ctrl+b a par défaut) et herdr plugin action invoke open --plugin ank. Une session herdr réelle est mesurée et consignée par ank log : après setup sur une copie du config.toml, ctrl+b a ouvre l'overlay. CI verte sur les trois OS.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
schema: 4
version: 2
---

## Pourquoi

Le manifeste de plugin de herdr 0.9.1 ne déclare aucun raccourci : les
[[keys.command]] et la disposition de la sidebar ne vivent que dans le
config.toml de l'utilisateur, et un plugin installé n'y écrit rien. Aujourd'hui
l'utilisateur copie les blocs du README à la main ; le README renvoie en plus à
une « palette de commandes » que herdr 0.9.1 n'a pas, si bien qu'Ouvrir ank
paraît introuvable.

## Choix

- Une commande explicite, lancée une fois par l'utilisateur, et non install.sh :
  un script d'installation qui modifie la config sans le demander est intrusif,
  et herdr plugin install le lance sans que l'utilisateur voie ce qu'il écrit.
- Ajout seulement, jamais de réécriture : une clé déjà posée par l'utilisateur
  gagne toujours, setup dit ce qu'il n'a pas fait.
- Sauvegarde avant écriture et restauration si herdr config check refuse le
  résultat.
