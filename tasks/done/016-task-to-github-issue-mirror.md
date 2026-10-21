---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
closed: 2026-10-21
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

1. [x] `python -I scripts/sync-issues.py --version` against the task standard's template; replace the
       script from the template only when the repo's copy is older
2. [x] `.github/workflows/sync-issues.yml` from the template: `main`, `tasks/**`, the issues-write
       token scoped to that job; `uvx zizmor .github` clean
3. [x] `python -I scripts/sync-issues.py --dry-run` (`gh` authenticated, `--dry-run` writes nothing):
       the summary and every `skip` line in the Log
4. [x] Docs under *Docs that move*; `python -I scripts/changelog.py --check`; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `gh repo view`: `speti4/mrgl`, public, default branch `main`. The repo's
  `scripts/sync-issues.py` (from the scaffolding, task 001) had no `--version` flag; its diff against the
  template was the template's own next revision (a `TEMPLATE_VERSION` constant, `--version`, `--root`,
  `--branch`, the second tasks location, a `skip` line instead of an abort on an un-numbered file), so it was
  replaced by the template unchanged: `cmp` against the template is identical, `--version` prints `2`.
  `scripts/changelog.py` imports the parser from it; `python -I scripts/changelog.py --check` still passes
- 2026-10-21 — `.github/workflows/sync-issues.yml` is the template as it is: `branches: [main]`, the first
  `paths:` entry `tasks/**` (the script is the second), `issues: write` only on the `sync` job, the checkout
  pinned to the same SHA as the other workflows with `persist-credentials: false`. `uvx zizmor .github`:
  "No findings to report" (1 ignored, 10 suppressed, as before)
- 2026-10-21 — `python -I scripts/sync-issues.py --dry-run`: `planned (dry run): created 36, updated 0,
  unchanged 0; 36 items (31 tasks, 5 decisions)`, plus `milestone  create` for the six roadmap headings
  (`M1 Core + CLI` … `M6 macOS preview`); no `skip` line, no orphan, no error. 001, 002 and 018 plan as
  closed issues, 016 as `stage:wip`, the five decisions as closed `decision` issues
- 2026-10-21 — docs: the mirror line in the `tasks/README.md` preamble (above the index markers), in
  `CLAUDE.md` under "Tasks", and in `docs/development.md` ("CI and releases" names the workflow, a new
  "The issue mirror" section says what it writes and that an issue is never edited by hand)

## Deviations from the plan

- Step 1 planned three outcomes for `--version` (equal, older, no version); the repo's copy had no version
  flag at all, which the standard reads as a hand-made copy to leave alone. The diff showed it was the
  template's previous revision instead, so it was replaced (the Log has the evidence); a copy without a
  version would otherwise never receive a template update.

## Verification

- 2026-10-21 — on the branch: `cmp scripts/sync-issues.py <template>` identical, `python -I
  scripts/sync-issues.py --version` → `2`; `diff .github/workflows/sync-issues.yml <template>` identical;
  `uvx zizmor .github` clean; `python -I scripts/changelog.py --check` passes; the dry run plans 6
  milestones and 36 issues with no `skip` line (the Log). Nothing was written to GitHub from the branch:
  `gh issue list --state all` and the milestones endpoint are still empty.
- The backfill is verified on `main` after the merge by the main session: the first `sync-issues` run
  reports `created 36, updated 0` (every file has its issue, six milestones), and a second run (`gh
  workflow run sync-issues.yml`, or `python -I scripts/sync-issues.py --check` from a logged-in `gh`)
  reports `created 0, updated 0`.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
