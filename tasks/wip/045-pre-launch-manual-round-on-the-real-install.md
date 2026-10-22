---
size: S
origin: done/013-wtf-backup-and-restore.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 045 — Pre-launch manual round on the real install: the commands no fixture covers

## Context

One dated round on the real game folders before the Forever launch, under the one-at-a-time rule, written as
this task's Verification without local paths: `add <zip>` and `remove` (004) on the real `_forever_` folder,
once with the game on another drive than the mrgl home; `backup`, `--list` and `--restore` (013) on a
flavor with a real `WTF`; `scan` and `list --json` with the client running (023); `scan` on the era folder
without a `.build.info` row (019); `add github:… --dry-run` (003) against a real repository with a Forever
release; `mrgl doctor` (024); `link` and `unlink` (005) with a real development folder; `savedvars` (025) on
a real `WTF`, its `Blizzard_` and letter-case rules; and whether the client loads an addon whose required
dependency is only a library embedded in its own folder (029's rule). The fixtures prove the code; these
touch the real folders, registry, API and client, before launch day (008).
Done when: every command above has a dated line in Verification, and every failure is a task.

Decided 2026-10-22: run with the release build of the CLI after the merge of tasks 064 to 072, one command
group per step; a failure is written here as a line and becomes a task through the wave's follow-up list.

## Docs that move

- none (a failure's fix is its own task, with its own docs)

## Not in this change

- The launch-day round on the live Forever client (008); any code change (each failure is a task of its own);
  the window (its manual check is part of 010's merge).

## Steps

1. [ ] `doctor`, `scan` on every flavor, `scan` on the era folder's build line (019), `list --json`
2. [ ] `add github:… --dry-run` against a real repository with a Forever release (003)
3. [ ] `add <zip>` and `remove` on the real `_forever_` folder, once with the mrgl home on another drive
4. [ ] `backup`, `backup --list`, `backup --restore` on a flavor with a real `WTF` (013)
5. [ ] `scan` and `list --json` with the client running (023)
6. [ ] `link` and `unlink` with a real development folder (005)
7. [ ] `savedvars` on a real `WTF`: the `Blizzard_` and the letter-case rules (025)
8. [ ] the embedded-library dependency on the client (029's rule); every failure listed; close

## Log

- 2026-10-22 — promoted; the round runs in the main checkout on its own branch, guided step by step

## Deviations from the plan

- none yet

## Verification

- (one dated line per command, without local paths)

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
