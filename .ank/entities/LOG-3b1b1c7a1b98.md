---
id: LOG-3b1b1c7a1b98
type: log
title: "review. Against the criterion: manifest [[build]] = sh install.sh and every other command"
created: 2026-09-25T18:41:33Z
author: haksolot@omarchy/ank-1
scope:
  - install.sh
  - herdr-plugin.toml
  - tests/install.rs
  - README.md
  - .gitignore
about: TASK-53028f116db2
seq: 4
schema: 4
version: 1
---

 ./bin/herdr-ank (herdr-plugin.toml, test the_manifest_builds_...); install.sh set -eu, curl|wget, version from the manifest, four targets from uname -sm, URL + HERDR_ANK_RELEASE_BASE file://, sha256sum|shasum, bin/herdr-ank 755; cargo fallback + exit 1 naming URL and cargo; /bin in .gitignore (test bin_is_ignored_by_git); tests for valid archive, wrong sum with/without cargo, unknown target; README Install rewritten. Extra: an unknown platform also falls back to cargo when present (ADR-7aab: 'plateforme sans asset'), and the README dev section now builds then copies into bin/ since link runs ./bin/herdr-ank and install.sh would fetch the released binary rather than local changes. Against constraints (ank context on all five paths: ADR-7aab, ADR-6fb7): the [[events]]/[[startup]] daemon entries are unchanged but for the path; nothing found.
