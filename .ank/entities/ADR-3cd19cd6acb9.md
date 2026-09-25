---
id: ADR-3cd19cd6acb9
type: adr
slug: le-plugin-lit-et-crit-le-corpus-ank-uniquement-t
title: Le plugin lit et écrit le corpus ank uniquement à travers ank <verbe> --json
created: 2026-09-25T16:52:32Z
author: haksolot@omarchy
status: proposed
scope:
  - src/**
constraint: |
  Aucun code du plugin n'ouvre un fichier sous .ank/ ni un ref sous refs/ank/. Toute lecture passe par ank <verbe> --json --repo <chemin>, le champ contract est vérifié avant de lire le reste, un champ inconnu est ignoré, et le code de sortie (0 à 9) est routé avant tout parsing. ank check et ank review ne sont jamais appelés par le daemon ni sur un timer : un poll utilise status, find, show ou context.
schema: 4
version: 1
---

docs/integrating.md d'ank est explicite : l'état d'une tâche n'est pas dans son
fichier. Le claim vit dans `refs/ank/claims/<id>`, la preuve dans
`refs/ank/proof/<id>`, le journal dans des entités LOG à côté. Un lecteur qui
parcourt `.ank/` rapporte une tâche tenue comme libre, silencieusement. Le
binaire, lui, répond entier : `ank show --json` porte `coordination`, `log`,
`machinery`.

Le surface se décrit elle-même par `ank help --json` (contrat 1) : un document
peut gagner un champ, jamais en perdre. D'où le parsing tolérant et la lecture
de `contract` en premier. Les codes de sortie sont stables et portent le
routage : 3 « relis », 4 « prends autre chose », 6 état interdit, 7 prérequis
absent, 9 environnement.

`ank check` élague les refs de claim périmés : il écrit, et ce qu'il élague est
le plan de coordination, précieux. `ank review` parcourt l'historique git.
Ni l'un ni l'autre n'est un poll.

Les fixtures `crates/ank-cli/tests/golden-json/` du dépôt ank sont offertes aux
clients tiers ; le client du plugin se teste contre elles.
