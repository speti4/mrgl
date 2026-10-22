---
size: S
origin: done/098-adopt-apply-writes-sidecar-entries-without-a-reinstall.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 110 — The window's install paths warn in a folder the CurseForge app manages

## Context

The window's add and update actions warn in a flavor folder with a `.curseclient` marker, with the
core's line of 098 (the CurseForge app may still update or flag those folders), shown once per run on the
flavor's view, as `add`, `import` and `update` do on the command line.
Why: 098 added the warning to the three command-line paths and left the window's actions without it, so
two managers on one folder is said in one place and not the other.
Done when: a window test over a marked flavor folder shows the warning on add and on update, and an
unmarked folder shows none.

Decided 2026-10-22: the first of two window follow-ups of adopt on one branch (110, 109), on the branch
of 097 and 098.

## Docs that move

- `docs/development.md`: the window's actions and the marker warning

## Not in this change

- Removing the marker; the adopt view itself (109)

## Steps

1. [ ] `actions.rs`: the add and the update actions carry the core's marker warning once per run; the UI
       shows it on the flavor's view
2. [ ] Tests: a window test over a marked flavor folder (add and update) and an unmarked one; the Node tests
       where the UI renders the warning

## Log

- 2026-10-22 — promoted

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
