---
id: ADR-2df155dab780
type: adr
slug: le-premier-daemon-d-un-state-dir-lance-herdr-ank
title: Le premier daemon d'un state dir lance herdr-ank setup une fois, sans que l'utilisateur ait à le faire
created: 2026-09-27T07:29:38Z
author: haksolot@omarchy
status: proposed
scope:
  - src/daemon/**
  - src/setup.rs
constraint: |
  Au premier démarrage du daemon dans un state dir ($HERDR_PLUGIN_STATE_DIR), avant la première sync, le daemon exécute la logique de herdr-ank setup telle quelle : ajout seulement, jamais d'écrasement d'une clé ou d'un bloc de l'utilisateur, sauvegarde config.toml.bak-ank-<horodatage>, herdr config check avec restauration en cas de refus, puis herdr server reload-config, le tout par $HERDR_BIN_PATH (ADR-357c). Un marqueur dans le state dir, posé après la tentative quel qu'en soit le résultat, fait qu'elle n'a lieu qu'une fois : un bloc retiré ensuite par l'utilisateur n'est jamais remis. Si un bloc est laissé (conflit ou disposition existante) ou si la tentative échoue, le daemon envoie une notification herdr qui nomme ce qui n'a pas été écrit et renvoie à herdr-ank setup ; il continue normalement. Le [[build]] (install.sh, install.ps1) ne touche jamais le config.toml de herdr. La commande herdr-ank setup reste disponible à la main.
schema: 4
version: 1
---

## Contexte

TASK-0b72 a fait de `herdr-ank setup` une commande explicite, lancée une fois par
l'utilisateur, jugeant intrusif qu'un script d'installation modifie la config.
En pratique une installation simple (`herdr plugin install haksolot/herdr-ank`)
laisse le plugin sans raccourci pour Ouvrir ank et sans les lignes de sidebar
qui affichent les tokens : l'utilisateur conclut que le plugin ne marche pas
(mesuré le 2026-09-27 sur omarchy, herdr 0.9.1, plugin 0.2.4 : daemon, open et
work fonctionnent ; config.toml sans aucun bloc ank).

## Décision

Le setup se lance tout seul, une fois, au premier démarrage du daemon.

## Options écartées

- install.sh / install.ps1 : la documentation des plugins herdr dit que les
  commandes [[build]] ne reçoivent ni le contexte du plugin ni l'env du socket
  (pas de HERDR_BIN_PATH), qu'elles s'exécutent avant l'enregistrement, et
  qu'un échec avorte l'installation. Le 2026-09-27 l'install (07:17:24Z) a eu
  lieu serveur arrêté (démarré à 07:17:31Z) : pas de reload-config possible.
  Il faudrait en plus le doubler sur Windows.
- À chaque démarrage : setup est idempotent, mais remettrait des blocs que
  l'utilisateur a retirés exprès.

## Conséquences

Le daemon, lancé par [[startup]] (ADR-6fb7), a HERDR_BIN_PATH et le serveur ;
herdr plugin link et Windows sont couverts sans code de plus. Les installations
existantes, qui ont déjà le marqueur de bienvenue mais pas celui du setup,
reçoivent le setup au prochain démarrage du daemon.
