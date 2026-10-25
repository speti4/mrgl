---
size: S
origin: done/097-adopt-the-report-of-the-sources-of-an-addons-folder.md
created: 2026-10-22
started: 2026-10-25
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

1. [ ] `adopt --flavor retail|classic|era --json`, the counts into Verification
2. [ ] The decision line on fingerprint matching; close

## Log

- 2026-10-25 — promoted with 128

## Deviations from the plan

- none yet

## Verification

- (pending)

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
