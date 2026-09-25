---
id: LOG-bb24d345ebe5
type: log
title: "reproduce: 'herdr pane report-metadata $HERDR_PANE_ID --source ank:probe --token x=1 --ttl-ms 1000"
created: 2026-09-25T17:52:11Z
author: haksolot@omarchy/ank-2
scope:
  - src/herdr/**
  - tests/herdr_client.rs
about: TASK-b636b02c8411
seq: 3
schema: 4
version: 1
---

 | wc -c' prints 0 with exit 0 (herdr 0.9.1); the daemon then logs 'unreadable herdr answer: pane report-metadata: EOF while parsing a value at line 1 column 0'. Hypothesis: Client::run decodes stdout as {id,result} for every verb, including those whose result nobody reads; refuted if an empty stdout still fails once those verbs skip decoding
