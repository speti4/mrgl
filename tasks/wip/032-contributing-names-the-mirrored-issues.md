---
size: S
origin: done/016-task-to-github-issue-mirror.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 032 — CONTRIBUTING.md names the mirrored issues as the task files' public face

## Context

016 installed the one-way mirror of the task files into GitHub issues; the contributor guide does not say
so yet. One line each under "Start with an issue" and "Tasks and decisions" in `CONTRIBUTING.md`: the
`mirrored` issues are the task files' public face, comments on them are welcome, the file under `tasks/`
is what the maintainer edits, and an issue body is overwritten by the next sync. `tasks/README.md`,
`CLAUDE.md` and `CONTRIBUTING.md` then tell the same story.
Done when: the two sections carry the line, and the three files agree.

## Docs that move

- `CONTRIBUTING.md`: the two lines (this is the whole change).

## Not in this change

- Anything in the mirror script or workflow; the milestone-closing step (033).

## Steps

1. [ ] The line under "Start with an issue" (a contributor's view: comment on the issue, the file is the source)
2. [ ] The line under "Tasks and decisions" (the maintainer's view: edit the file, never the issue)
3. [ ] Read the three files side by side for one wording; `python -I scripts/changelog.py --check`; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, when the work is done)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the stale index is the expected finding on a task branch)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
