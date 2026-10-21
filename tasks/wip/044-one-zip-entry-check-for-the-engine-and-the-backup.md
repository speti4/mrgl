---
size: S
origin: done/013-wtf-backup-and-restore.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M3 Toolbox
changelog: housekeeping
---

# 044 — One zip-entry check for the install engine and the backup restore

## Context

The install engine's path check (004, `safe_relative_path`) and the restore's (013, `safe_entry_path`)
fold into one function with one `Error` pair (an unreadable zip, an escaping entry) that both call; the
two fixture tests stay and run against the shared check. Two copies of the one rule that keeps a zip
from writing outside its folder drift apart, and the hardening review of M5 wants one place to read.
Done when: one function, both callers, both escape tests green, and the duplicated error variants gone.

Decided 2026-10-21: the shared check lives in its own small module (`zip_entry.rs` or the name the
layout prefers), the stricter of the two rules wins wherever they differ (the difference is documented in
the module doc), and the user-facing messages of both commands stay as they are.

## Docs that move

- `docs/development.md` ("Layout"): the shared module, named by both the engine and the backup.

## Not in this change

- The bound on the unpacked size of a zip (039); any change to the engine's swap or the restore's
  moved-aside folder.

## Steps

1. [ ] The shared function and its error pair in one module; the two callers switched to it
2. [ ] The duplicated error variants removed; every message the two commands print unchanged (the CLI
       tests prove it)
3. [ ] The two fixture escape tests run against the shared check, plus one unit test per rule difference
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
