---
id: TASK-e46800b0edb8
type: task
slug: readme-exemple-de-sidebar-sujet-github-herdr-plu
title: README, exemple de sidebar, sujet GitHub herdr-plugin
created: 2026-09-25T16:54:29Z
author: haksolot@omarchy
status: done
scope:
  - README.md
  - herdr-plugin.toml
blocked_by: [TASK-8e7f97898a70, TASK-91a148032c7e, TASK-c81aba215f4d, TASK-b12178dca482]
done_criteria: |
  Le README décrit l'installation (`herdr plugin install haksolot/herdr-ank`, `herdr plugin link` pour le développement), les quatre fonctions avec la commande herdr qui les déclenche, chaque clé de config et son défaut, et un extrait de configuration herdr affichant `$ank_task` et `$ank_title` dans la sidebar Agents. `herdr plugin install haksolot/herdr-ank --yes` sur une machine avec cargo aboutit à `herdr plugin list --json` listant `ank`. Le dépôt GitHub porte le sujet `herdr-plugin`.
criteria_by: creator
proof:
  - type: assertion
    ref: herdr plugin install haksolot/herdr-ank --yes at 378611b lists ank (github source, enabled), 2026-09-25T18:14Z; repo topic herdr-plugin (gh repo view)
    criteria: ad531bf47cd6
    via: submitted
schema: 4
version: 5
---

Dernière tâche : rien à écrire tant que les fonctions ne sont pas là.
Le marketplace herdr indexe automatiquement le sujet `herdr-plugin` toutes
les 30 minutes, sans revue ; le README est ce qu'un utilisateur lira avant
de faire confiance au build.
