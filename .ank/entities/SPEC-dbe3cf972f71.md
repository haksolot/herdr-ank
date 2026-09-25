---
id: SPEC-dbe3cf972f71
type: spec
slug: ce-que-le-plugin-rapporte-herdr-tokens-de-pane-s
title: "Ce que le plugin rapporte à herdr : tokens de pane, source, ttl"
created: 2026-09-25T16:52:34Z
author: haksolot@omarchy
status: proposed
scope:
  - src/sync/**
references: [ADR-fa8b6a103597, ADR-357c017baf9b]
schema: 4
version: 1
---

Le plugin ne possède aucune surface d'affichage : ce qu'il montre, il le
confie à herdr par `pane report-metadata`. Ce document fixe ce qu'il envoie,
pour que la sidebar se configure contre des noms stables.

## Source

Toute métadonnée est rapportée avec `--source ank:sync`. Aucune autre source
n'est utilisée par le daemon, et le daemon ne touche jamais une métadonnée
d'une autre source.

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

## Titre de pane

Le plugin ne pose pas de `--title` : le titre appartient à l'agent et à
l'utilisateur. La sidebar affiche `$ank_task` et `$ank_title` par
configuration herdr, et le README montre l'exemple.

## Durée de vie

Chaque rapport porte `--ttl-ms 90000`, trois fois l'intervalle de poll : un
daemon mort laisse la sidebar se vider seule en moins de deux minutes plutôt
que d'afficher un claim fantôme.

## Panes sans corpus

Une pane dont le cwd n'est dans aucun corpus, ou qui n'héberge pas d'agent,
ne reçoit aucun rapport. Le daemon ne crée pas de métadonnée vide.
