---
size: M
origin: done/014-addon-sets-with-enable-and-disable.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 074 — The window's toolbox: a set switch and a SavedVariables view

## Context

Per flavor, a set picker that applies a set to every character or to one (`sets::apply_set` with a
`Target`), and a SavedVariables view with sizes, orphans and a cleanup button (`savedvars`), both refused
while the client runs with the voice's line. The toolbox commands of M3 exist in the CLI only; the window is
where most users will look for them.
Done when: on a fixture flavor the window applies a set and moves the orphans into the dated zip, and both
are refused with the stand-in client running.

Decided 2026-10-22: one task on its own branch in wave 6, built on the core's toolbox as it is today
(`sets::apply_set`, `sets::find_character`, `savedvars`); the set editing and the undo of 073 land in the
command line first and join the window later.

## Docs that move

- `docs/development.md`: the UI section (the toolbox panel, its pure modules and their tests) and the IPC
  commands (the set and SavedVariables commands).
- `README.md`: the window's feature list, where the toolbox is named.

## Not in this change

- Saving a set from the window; the set undo and the set editing (073); the `WTF` backup and restore in the
  window; a new online request.

## Steps

1. [ ] IPC commands in `mrgl-app/src/commands.rs` over plain functions of a new `src/toolbox.rs`: the sets of a
       flavor with their addons, the characters, `apply_set` to every character or one, the SavedVariables
       listing (sizes, orphans, client files) and the cleanup into the dated zip; every write runs the
       running-client guard (023) first and answers the refusal through the voice
2. [ ] `ui/`: a toolbox panel per flavor (the set picker with the target, the SavedVariables table with the
       cleanup button), its pure module(s) with Node tests under `tests/ui/`, the mock data for the sketch
3. [ ] Tests in `mrgl-app/tests/`: on a fixture flavor the window applies a set (every character and one) and
       moves the orphans into the dated zip; both refused with the stand-in client running
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
