---
id: SPEC-43438bbcb5ca
type: spec
slug: ce-que-le-plugin-rapporte-herdr-tokens-de-pane-e
title: "Ce que le plugin rapporte à herdr : tokens de pane et de workspace, libellé d'agent, source, ttl"
created: 2026-09-26T06:55:22Z
author: haksolot@vmi3223161
status: accepted
scope:
  - src/sync/**
references: [ADR-fa8b6a103597, ADR-357c017baf9b]
supersedes: SPEC-dbe3cf972f71
ratified: 2a7112a955ba
verified:
  - by: haksolot@vmi3223161/ank-4fbf
    at: 2026-09-26T09:02:08Z
schema: 4
version: 2
---

Le plugin ne possède aucune surface d'affichage : ce qu'il montre, il le
confie à herdr par `pane report-metadata` et `workspace report-metadata`. Ce
document fixe ce qu'il envoie, pour que la sidebar se configure contre des noms
stables, et pour qu'une installation sans configuration montre déjà que ank
suit une pane.

Remplace SPEC-dbe3cf972f71. Ce qui change : le libellé d'agent
(`--display-agent`), rapporté sans configuration, et les tokens de workspace.

## Source

Toute métadonnée, de pane comme de workspace, est rapportée avec
`--source ank:sync`. Aucune autre source n'est utilisée par le daemon, et le
daemon ne touche jamais une métadonnée d'une autre source.

## Tokens par pane

Pour une pane qui héberge un agent et dont le cwd appartient à un worktree
portant un corpus `.ank/` :

| token | valeur | absent quand |
|---|---|---|
| `ank_task` | l'id court de la tâche tenue, `TASK-6da1` | aucun claim attribué |
| `ank_title` | le titre de la tâche, tronqué à 60 caractères | aucun claim attribué |
| `ank_expires` | minutes avant expiration du claim, entier | aucun claim attribué |
| `ank_ambiguous` | `1` si le claim est tenu par l'identité de repli et attribué à plusieurs panes | attribution univoque |
| `ank_queue` | nombre de tâches claimables dans ce corpus | jamais, dès qu'il y a un corpus |

Un token absent est effacé par `--clear-token`, jamais laissé à une valeur
périmée.

## Libellé d'agent

La même pane reçoit `--display-agent`, que la ligne `agent` par défaut de la
sidebar herdr affiche sans aucune configuration :

- avec un claim attribué : `<agent> · <id court>`, par exemple
  `claude · TASK-6da1` ;
- sans claim attribué : `<agent> · ank`.

`<agent>` est le nom d'agent que herdr rapporte pour la pane (champ `agent` de
`pane list`), celui qu'ADR-fa8b6a103597 lit pour attribuer un claim. Le libellé
est un affichage : il ne remplace jamais ce nom, et l'attribution continue de
le lire. Le séparateur est ` · ` (espace, U+00B7, espace).

## Tokens par workspace

Un workspace dont au moins une pane a un cwd dans un corpus reçoit, par
`workspace report-metadata`, les tokens du corpus qui possède le plus de ses
panes ; en cas d'égalité, celui de la première de ces panes dans l'ordre de
`pane list`.

| token | valeur |
|---|---|
| `ank_queue` | nombre de tâches claimables dans ce corpus |
| `ank_claims` | nombre de claims vivants dans ce corpus |
| `ank_review` | nombre de décisions en attente de ratification |

Ces trois tokens sont toujours présents sur un tel workspace, `0` compris.
Un workspace sans pane dans un corpus ne reçoit aucun rapport.

## Titre de pane

Le plugin ne pose pas de `--title` : le titre appartient à l'agent et à
l'utilisateur.

## Durée de vie

Chaque rapport, tokens et libellé, de pane comme de workspace, porte
`--ttl-ms 90000`, trois fois l'intervalle de poll maximal : un daemon mort
laisse la sidebar revenir d'elle-même à l'affichage de herdr en moins de deux
minutes plutôt que d'afficher un claim fantôme.

## Panes sans corpus

Une pane dont le cwd n'est dans aucun corpus, ou qui n'héberge pas d'agent,
ne reçoit aucun rapport, ni token ni libellé. Le daemon ne crée pas de
métadonnée vide.
