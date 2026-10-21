---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 025 — SavedVariables: size per addon and orphan cleanup into a backup zip; CLI `savedvars`

## Context

`mrgl savedvars --flavor <id>` lists the `SavedVariables\<Addon>.lua` sizes per account and per
character, and marks orphans (a file whose addon folder no longer exists under `Interface\AddOns`; a disabled
addon keeps its folder, so it is never an orphan). `--clean` moves the orphans and their `.bak` files into a
dated zip next to the backups of 013, never deletes, and refuses while the game runs (023). The third `WTF`
write of D-002 and D-003 (amended 2026-10-21). The classic `WTF` bloat: megabytes of settings for addons
removed months ago, and a 50 MB file behind a long loading screen.
Done when: a fixture `WTF` folder lists sizes and orphans correctly, `--clean` round-trips the moved files
through the zip byte for byte, and nothing outside `SavedVariables` is touched.

Decided 2026-10-22: the cleanup zip is `backups\<flavor-id>\SavedVariables-<date>.zip`, written with the
backup module's zip writer (013), and `backup --restore` is not taught to read it (a restore of orphans
is an unzip by hand, documented).

## Docs that move

- `README.md` ("What it will do") and `docs/development.md` ("Layout", the home folder's files): the
  command, the orphan rule and the zip's name.

## Not in this change

- A size bound or a warning on a large file in `list`; the window; a restore of the cleanup zip.

## Steps

1. [ ] The SavedVariables walk in the core (account-wide and per-character files, sizes, the orphan rule
       against the AddOns scan)
2. [ ] `--clean`: the orphans and their `.bak` files into the dated zip, moved not copied, the `WTF` write
       guard of 023 before the move
3. [ ] The CLI command with the voice's lines and `--json`
4. [ ] Tests on a fixture `WTF`: the sizes, the orphans (a disabled addon is not one), the byte-for-byte
       round trip through the zip, the refusal while the stand-in client runs
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

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
