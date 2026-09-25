---
id: TASK-4e8fd40b787a
type: task
slug: install-ps1-amorce-herdr-ank-exe-depuis-la-relea
title: install.ps1 amorce herdr-ank.exe depuis la release, le manifeste déclare windows, le workflow publie la cible windows-msvc
created: 2026-09-25T19:06:39Z
author: haksolot@omarchy
status: open
scope:
  - install.ps1
  - herdr-plugin.toml
  - .github/workflows/release.yml
  - tests/install_ps1.rs
  - README.md
  - tests/install.rs
blocked_by: [TASK-678211d8548c, TASK-194a3122f31c]
done_criteria: |
  `herdr-plugin.toml` déclare `platforms = ["linux", "macos", "windows"]`, un `[[build]]` `["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "install.ps1"]` avec `platforms = ["windows"]`, et l'ancien `[[build]]` restreint à linux et macos ; si herdr n'ajoute pas `.exe` lui-même en résolvant `./bin/herdr-ank` (le log cite le code de herdr, github.com/herdrdev/herdr, qui tranche), chaque entrée startup, actions, panes et events a une variante `platforms = ["windows"]` pointant sur `./bin/herdr-ank.exe`. `install.ps1` (Windows PowerShell 5.1, TLS 1.2 explicite) lit la version du manifeste, télécharge `herdr-ank-<version>-x86_64-pc-windows-msvc.zip` et `SHA256SUMS` depuis la release v<version> (base remplaçable par HERDR_ANK_RELEASE_BASE, qui accepte un chemin de répertoire local), vérifie la somme avec Get-FileHash, extrait `herdr-ank.exe` dans `bin\`, se rabat sur `cargo build --release` si cargo est présent, sinon sort 1 en nommant l'URL et l'absence de cargo. `release.yml` ajoute la cible x86_64-pc-windows-msvc sur windows-latest, archive en .zip, et `SHA256SUMS` couvre les cinq archives. `tests/install_ps1.rs` (cfg(windows)) lance le script contre un répertoire local : archive valide, somme fausse, cargo absent du PATH. Preuve : job windows du CI vert pour ce test, URL dans le log ; actionlint ne rapporte rien.
criteria_by: creator
verify: [cargo-test, clippy, fmt-check]
method: tdd
schema: 4
version: 4
---

Applique ADR-599b6f424271. Le contrat d'install.sh est la référence, jusqu'aux
codes de sortie ; install.ps1 d'ank (dans le dépôt ank) est un bon modèle de
style pour PowerShell 5.1.
