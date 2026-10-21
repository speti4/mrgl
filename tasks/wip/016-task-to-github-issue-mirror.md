---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 016 — Mirror the task files into GitHub issues and milestones

## Context

A public project's contributors look at issues; the task files stay the only place to edit. The task
standard's `mirror` makes every numbered task and decision one issue (found again by a hidden marker, so a
re-run patches only what differs) and every `## ` heading of `tasks/roadmap.md` a milestone, one way:
nothing flows back, an issue is never edited by hand. `scripts/sync-issues.py` is already in the repo; the
workflow `.github/workflows/sync-issues.yml` runs it on every push to `main` that touches `tasks/`, and that
first run on `main` is the backfill, so no issue is created by hand from a branch.
Done when: every task and decision file has its issue, the milestones match the roadmap, and a second run
reports nothing to change.

Decided 2026-10-21: the runner is CI (the repo is public, the Actions minutes are free); the backfill
happens on the merge of this change, never from a task branch.

## Docs that move

- `tasks/README.md` preamble (above the generated markers): one line that every numbered file is mirrored
  one way into an issue, edit the file, never the issue.
- `CLAUDE.md` ("Tasks"): the mirror line.
- `docs/development.md` ("CI and releases"): the `sync-issues` workflow and what it writes.

## Not in this change

- Anything under `crates/`.
- Editing an issue by hand, or a backfill run from the branch.

## Steps

1. [ ] `python -I scripts/sync-issues.py --version` against the task standard's template; replace the
       script from the template only when the repo's copy is older
2. [ ] `.github/workflows/sync-issues.yml` from the template: `main`, `tasks/**`, the issues-write
       token scoped to that job; `uvx zizmor .github` clean
3. [ ] `python -I scripts/sync-issues.py --dry-run` (`gh` authenticated, `--dry-run` writes nothing):
       the summary and every `skip` line in the Log
4. [ ] Docs under *Docs that move*; `python -I scripts/changelog.py --check`; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, when the work is done; the backfill result is verified on `main` after the merge)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the stale index is the expected finding on a task branch)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
