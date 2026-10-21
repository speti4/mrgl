---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 006 — Update check: installed state against the latest release per flavor; `update --all` with dry-run and `--yes`

## Context

Compare every managed addon's sidecar entry with its source's latest release for the flavor, report
up to date / update available / source gone / guess only, and `mrgl update --all [--dry-run] [--yes]` apply
the updates with one confirmation, each through the one install helper of 055 with the source's
download (003) and the pick's version as the metadata. The voice's progress and summary lines (`2 addons
updated, 12 up to date`); an ignored addon is skipped, a pinned one reported and left alone, each addon
resolved on its own channel (021), and the summary counts pinned and ignored addons separately. The daily
command of the first weeks after launch.
Done when: a fixture sidecar against fixture releases reports every state, `--dry-run` writes nothing, and
the summary line counts are right.

Decided 2026-10-22: second in the chain 055 → 006 → 057; `update <name>` (one addon) takes the same path
as `--all` over one entry, and the stub of `update` that named this task goes.

## Docs that move

- `README.md` ("What it will do") and `docs/development.md`: the `update` command, its flags and the states
  it reports.

## Not in this change

- The window's update button and progress (010); the offline resolve from the cache (041); the GitHub
  pagination and rate-limit edges (042); a per-addon rollback after a bad update (022 exists).

## Steps

1. [ ] The update check in the core: every managed entry against its source's latest pick on the entry's
       channel, one state per addon (up to date, update available, source gone, guess only, pinned, ignored)
2. [ ] `mrgl update --all [--dry-run] [--yes]` and `mrgl update <name>` through the install helper of 055;
       progress and summary lines through the voice, `--json`
3. [ ] Tests: a fixture sidecar against fixture releases reports every state; `--dry-run` writes nothing;
       the summary counts; a pinned and an ignored entry
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 055 → 006 → 057

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, at close)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
