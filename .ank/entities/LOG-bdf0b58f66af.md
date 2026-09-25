---
id: LOG-bdf0b58f66af
type: log
title: "herdr 0.9.1 measured: no subcommand offers --json (CLI prints JSON always: {id,result} on stdout"
created: 2026-09-25T17:20:20Z
author: haksolot@omarchy/ank-2
scope:
  - src/herdr/**
  - tests/herdr_client.rs
  - tests/fixtures/herdr/**
about: TASK-de0a51842971
seq: 3
schema: 4
version: 1
---

 exit 0, {id,error{code,message}} on stderr exit 1); so ADR-357c's 'with --json when offered' passes nothing
