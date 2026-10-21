---
status: decided
date: 2026-10-21
cost: medium
sources: [wip/001-project-scaffolding.md, docs/research/2026-10-21-addon-manager-alternatives.md, docs/origin.md]
---

# D-001 — Build our own addon manager instead of adopting one

## Context

Four World of Warcraft flavors side by side, with `_forever_` (launching 2026-11-04) needing addons of its
own, and a manager that must pick the Forever build of each addon rather than "latest". The existing
managers were measured against that need on 2026-10-21 (`docs/research/2026-10-21-addon-manager-alternatives.md`):
none combine a flavor-correct pick, a CurseForge-free source set, a portable zip with nothing to install and
a maintained code base.

## Decision

Build our own manager, `mrgl`: a Rust core, a command line and a window, with the sources and the stack of
D-002. The cost is medium: one milestone of work before it is useful, and the research can be reused if the
project is dropped. Decided 2026-10-21.

## Not chosen

- CurseForge app: Electron with Overwolf heritage, owns the AddOns folder, picks "latest" and is flavor-blind
  for Forever; no API for other tools.
- WowUp: Electron, GPL-3.0, Forever support unconfirmed; CurseForge only through its separate CurseForge
  edition since 2022.
- Ajour: the one Rust manager, GPL-3.0, archived 2024-09-17 after the CurseForge API restriction.
- instawow: a Python CLI with a runtime to install, no Forever flavor.
- Strongbox: Clojure on a JVM, Linux-first.
- forever-addon-manager: an early Python/Tk script, one flavor, no state, GitHub as the only source.
