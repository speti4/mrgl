---
size: S
origin: done/016-task-to-github-issue-mirror.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
closed: 2026-10-21
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

1. [x] The line under "Start with an issue" (a contributor's view: comment on the issue, the file is the source)
2. [x] The line under "Tasks and decisions" (the maintainer's view: edit the file, never the issue)
3. [x] Read the three files side by side for one wording; `python -I scripts/changelog.py --check`; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — the label name checked against the script: `MIRROR_LABEL = "mirrored"` in
  `scripts/sync-issues.py`, and the issue body it writes says "edit the file, not this issue; comments are
  welcome here". The three files and `docs/development.md` ("The issue mirror") read side by side: all say
  mirrored one way, edit the file, never the issue; `docs/development.md` also says a comment on an issue is
  welcome, `CLAUDE.md` that the H1 and Context are the issue text. The two new lines in `CONTRIBUTING.md`
  use those words: under "Start with an issue" the contributor's view (the `mirrored` issues are the task
  files' public face, one numbered file is one issue, a comment is welcome, the file is what the maintainer
  edits, the next sync overwrites the issue body), under "Tasks and decisions" the maintainer's view (the
  script, the label, one way, a link to the mirror section of `docs/development.md`, edit the file, never
  the issue, the next sync overwrites the issue body). No other file needed a change.

## Deviations from the plan

- The Context names three files; `docs/development.md` was read as a fourth, since its "The issue mirror"
  section is the long form the new line links to. It already agreed, so the change stays `CONTRIBUTING.md`
  only.

## Verification

- 2026-10-21 — on the branch: `git diff` shows `CONTRIBUTING.md` as the only changed file besides this
  task file and the changelog; the paragraph under "Start with an issue" and the sentence under "Tasks and
  decisions" are in place; `grep -n -i mirror CONTRIBUTING.md tasks/README.md CLAUDE.md docs/development.md`
  finds the mirror line in every one of the four, and `grep -n MIRROR_LABEL scripts/sync-issues.py` shows
  the label is `mirrored`; the anchor `#the-issue-mirror` matches the `### The issue
  mirror` heading of `docs/development.md`; `python -I scripts/changelog.py --check` passes; `tasks_check`
  without `write` reports 0 errors, 0 warnings.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
