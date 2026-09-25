---
id: ADR-357c017baf9b
type: adr
slug: le-plugin-commande-herdr-par-la-cli-herdr-bin-pa
title: Le plugin commande herdr par la CLI $HERDR_BIN_PATH et n'utilise le socket que pour events.subscribe
created: 2026-09-25T16:52:33Z
author: haksolot@omarchy
status: accepted
scope:
  - src/**
constraint: |
  Toute action sur herdr (report-metadata, notification show, worktree create, tab create, agent start, agent prompt, plugin pane open) passe par le binaire $HERDR_BIN_PATH avec --json quand la sous-commande l'offre. Le socket $HERDR_SOCKET_PATH est ouvert pour une seule méthode, events.subscribe, lue en NDJSON. Les champs inconnus d'une réponse sont ignorés et une méthode non supportée est une erreur ordinaire, jamais un panic.
ratified: b54874323e3f
verified:
  - by: haksolot@omarchy
    at: 2026-09-25T17:11:52Z
schema: 4
version: 2
---

La documentation des plugins herdr désigne la CLI comme surface première
(« Plugins call Herdr through HERDR_BIN_PATH using the entire Herdr CLI ») et
le socket JSON comme voie avancée. La CLI est versionnée avec le serveur qui
l'a lancée, elle résout elle-même le socket de session, et chaque sous-commande
est documentée par `herdr <domaine>`.

Une chose que la CLI ne fait pas : rester abonnée à un flux. `events.subscribe`
est une requête qui garde la connexion ouverte et pousse des événements ; c'est
la seule raison d'ouvrir le socket, et la seule méthode qu'on y parle. Tenir la
frontière nette évite de maintenir deux clients pour un protocole (22 au moment
de la décision) dont la doc demande d'ignorer les champs inconnus.
