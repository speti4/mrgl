---
size: S
origin: session 2026-10-25
created: 2026-10-25
started: 2026-10-25
closed: 2026-10-25
plan: none
milestone: M5 Trust
changelog: housekeeping
---

# 127 — The issue mirror closes its duplicate issues

## Context

`scripts/sync-issues.py` closes a second issue of a task or decision that already has one (the same marker
key; the lowest issue number is the one kept) as not planned, with a comment naming the issue that stays; a
true orphan, an issue whose file is gone, is still reported and left alone. One mirror run of 2026-10-22
created a second issue for every item that existed then, and the runs since report them as orphans, so 11
stale issues stay open with backlog labels and 88 closed ones clutter the list.
Why: the issue list is the task files' public face; a duplicate with stale labels says the wrong thing about
a task, and the repair has to be the script's, so the next run fixes it wherever it happens again.
Done when: a dry run on the repo plans the 11 closes and no create, the workflow run after the merge closes
them, `--check` passes afterwards, and the project-pal template carries the rule under a new template version.

Decided 2026-10-25: the rule lives in the project-pal template (the repo's copy is never edited on its own: a
change goes through the template and a version bump, as the standard's mirror rule says), and nothing is closed
by hand: the workflow run that follows the merge applies it. The cause of the one run that created the
duplicates is not established (its log is gone); the rule repairs the result whatever the cause.

## Docs that move

- `docs/development.md`, "The issue mirror": a duplicate issue is closed, an orphan is left alone
- the script's docstring (the template and the copy)

## Not in this change

- Why the run of 2026-10-22 saw no existing issue (not recoverable; a guard against a bulk create would be
  a change of its own if it happens again)
- Tests of `sync-issues.py` (left out by 122 on purpose; the dry run on the real repo is the check here)

## Steps

1. [x] The template in project-pal: `match_issues` keeps the lowest-numbered issue of a key; after the match
       the leftovers split into duplicates (a key an item has) and orphans; an open duplicate is closed as
       not planned with a comment naming the kept issue, a closed one is logged; `TEMPLATE_VERSION` 3
2. [x] The copy in `scripts/sync-issues.py` (byte-identical to the template), `docs/development.md`
3. [x] `python -I scripts/sync-issues.py --dry-run`: 11 closes planned, 88 closed duplicates logged, nothing
       created; the checks of CONTRIBUTING; close

## Log

- 2026-10-25 — promoted
- 2026-10-25 — the template: the issues of one marker key are kept as a list sorted by number and
  `match_issues` takes the first, so the issue a run keeps no longer depends on the order the API lists them
  in (the older one won by accident before: it came last in the listing and overwrote the dict entry); the
  leftovers are split by whether an item has the key (`same_id`, so a padded id counts): a duplicate is closed
  as not planned after one comment, `Duplicate of #N, created by a second mirror run; the task's issue is
  #N.`, a closed duplicate is logged as `already closed`, an orphan is logged as before. The summary line
  gains `duplicates closed N`, which `--check` counts as a change. `TEMPLATE_VERSION` is `"3"`; the copy here
  is the template byte for byte (`cmp`). The standard's mirror paragraph in project-pal names the rule.
- 2026-10-25 — the run of 2026-10-22 15:46 (the push of #146) created #149 to #249, one for each of the 99
  items that existed then; its log is no longer served, so why its listing came back empty stays unknown.

## Deviations from the plan

- none

## Verification

- 2026-10-25 — `python -I scripts/sync-issues.py --dry-run --repo speti4/mrgl` on this branch: `planned (dry
  run): created 1, updated 0, unchanged 125, duplicates closed 11; 126 items (120 tasks, 6 decisions)`; the
  one create is this task's own issue, the 11 closes are #155, #156, #179, #184 (tasks 007, 008, 031, 036,
  kept #14, #15, #38, #51) and #236 to #242 (tasks 094 to 100, kept #139 to #143, #147, #148), and 88
  `already closed` lines name the closed duplicates. `--version` prints 3. `python -I scripts/usage-guide.py
  --check`, `python -I -m unittest discover -s scripts/tests` and `cargo fmt --check` pass; no Rust file
  changed, so the Cargo checks are CI's on the pull request.
- The apply is the `sync-issues` run after the merge: the 11 closed, then `--check` exits 0 (the line for
  it goes into the pull request after the merge).

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: 2026-10-25`
