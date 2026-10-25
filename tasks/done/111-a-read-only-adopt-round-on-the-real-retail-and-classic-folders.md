---
size: S
origin: done/097-adopt-the-report-of-the-sources-of-an-addons-folder.md
created: 2026-10-22
started: 2026-10-25
closed: 2026-10-25
plan: none
milestone: M4 Sources
---

# 111 — A read-only adopt round on the real retail and classic folders

## Context

`mrgl adopt --flavor` run once, read-only, on the maintainer's real retail, classic and era folders (managed
by the CurseForge app today), the summary counts recorded in the task file without any addon or folder name:
how many addons adopt without CurseForge, how many only with it, how many have no known source, and how many
of the hinted sources answer "needs <KEY NAME>".
Why: the counts measure the roadmap's M4 goal (most addons of the retail and classic folders recognized)
and decide whether fingerprint matching is worth a task of its own; no write and no key is needed.
Done when: the three counts per folder are in the task file's Verification and a decision line says whether
fingerprint matching is filed or dropped.

Decided 2026-10-25: taken as step 4 of the manual round of 128, on the same branch.

## Docs that move

- none

## Not in this change

- `adopt --apply` on the three folders (they stay with the CurseForge app); fingerprint matching itself

## Steps

1. [x] `adopt --flavor retail|classic|era --json`, the counts into Verification
2. [x] The decision line on fingerprint matching; close

## Log

- 2026-10-25 — promoted with 128
- 2026-10-25 — the three reports taken read-only with the release build, each in a few seconds (retail and
  classic with a GitHub token, since the anonymous budget was spent by the round's installs); every folder
  carries the CurseForge app's marker, so each report ends with the one-manager line and `--apply` would
  write nothing without `--alongside-curseforge` (checked on era: nothing written, exit 1)

## Deviations from the plan

- none

## Verification

- 2026-10-25 — the counts of `mrgl adopt --json` per folder (unmanaged addons; adoptable without CurseForge /
  only with CurseForge / no known source / no build for the flavor at any source; the addons `--apply` would
  adopt now; the addons with a hinted source that answers `needs <KEY NAME>`):

  | folder | unmanaged | without CurseForge | only CurseForge | no source | no build | apply adopts | needs a key |
  |---|---|---|---|---|---|---|---|
  | retail | 64 | 30 | 7 | 22 | 5 | 23 | 14 (CurseForge 14, Wago 7) |
  | classic | 64 | 30 | 5 | 24 | 5 | 23 | 12 (CurseForge 12, Wago 7) |
  | era | 46 | 9 | 2 | 30 | 5 | 7 | 4 (CurseForge 4, Wago 2) |

  The hinted sources (an addon may name several): retail 36 CurseForge, 30 Wago, 22 WoWInterface, 3 GitHub;
  classic 35, 30, 21, 6; era 10, 13, 3, 5. So about half of the retail and classic addons and a fifth of era's
  adopt today without CurseForge; a third to two thirds name no source at all (no `X-…-ID` header and no
  GitHub mapping), and those are what fingerprint matching would recognize.
- Decided 2026-10-25: fingerprint matching is not filed. The M4 goal stands on the TOC ids and the GitHub
  mapping: about half of the retail and classic addons adopt without CurseForge today, and most of the rest
  name a CurseForge id the user's own key resolves (096); the addons with no source at all are the long
  tail, and a CurseForge-only matching of them is not worth a task while the key application (007) waits.
  Reopen when the numbers above change or the key is in hand and the no-source share still matters.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: 2026-10-25`
