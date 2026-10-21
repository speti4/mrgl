---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 022 — Rollback to the previous version from the kept zip; CLI `rollback`

## Context

`mrgl rollback <name>` installs the previous version's zip the install engine (004) kept in the cache
(`%LOCALAPPDATA%\mrgl\<flavor-id>\cache\<addon>\<version>.zip`, one previous version per addon) with the
same atomic swap (the engine's install call with the cached zip and the previous version as the
metadata), and pins the addon at that version (021). The swap's own revert on a failed install in 004 is
not this command. With daily broken builds after launch, "put yesterday's back" is one command instead of
a hunt for a zip.
Done when: an install, an update and a rollback in a temporary AddOns folder leave the first version's
files byte for byte, the sidecar shows the pin, and a rollback with no kept zip is a clear error.

Decided 2026-10-21: a rollback pins, so the next `update --all` does not reinstall the broken build;
`unpin` is the deliberate step back. The rollback reads the entry's `previous_version` and the cached zip
for it; the entry's source is kept; after the rollback the cache holds the rolled-back version (now
current) and the version rolled back from (now previous), so a second rollback goes forward again.

## Docs that move

- `README.md`: the `rollback` usage line next to `pin` and `unpin`.
- `docs/development.md` ("Layout"): the rollback path through the engine and the cache.

## Not in this change

- `update --all` (006), which reads the pin; a rollback across more than one version; the window's row
  action (010).

## Steps

1. [ ] The core rollback: the entry's previous version and its cached zip, the engine's install call with
       the cached zip and that version as the metadata, the pin on the entry, the sidecar written
2. [ ] Errors: not a managed addon, no previous version, the cached zip missing (a clear line each)
3. [ ] `mrgl-cli`: `rollback <name> --flavor <id>`, the voice lines, `--json`
4. [ ] Tests: install 1.0.0, update to 1.1.0, rollback; the files are 1.0.0's byte for byte, the sidecar
       pinned at 1.0.0, the cache holds both; a rollback with no kept zip is the documented error
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), on top
  of the branch of task 021 (the pin)

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
