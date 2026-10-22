---
size: S
origin: done/084-stale-staging-folders-are-swept.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 088 — The doctor names a staging folder kept for its `previous\` folders

## Context

`mrgl doctor` lists every run folder under `<flavor-id>\staging\` that the sweep of 084 keeps because
it holds a `previous\` folder (the addon folders a failed swap moved aside), with its age and size, and says
what to do with it.
Why: after the error of a failed revert nothing points the user to that folder again, and the sweep never
removes it.
Done when: a planted run folder with a `previous\` folder shows in the doctor's text and JSON, and one
without it does not.

Decided 2026-10-22: the first of three wave-7 follow-ups on one branch (088, 089, 090), in id order.

## Docs that move

- `docs/development.md`: the doctor's checks (the kept staging folders line and its JSON)

## Not in this change

- Removing or restoring the kept folder for the user (the doctor names it, the user decides)
- The sweep rules of 084

## Steps

1. [ ] `doctor.rs`: a check over `<flavor-id>\staging\` listing every run folder with a `previous\` folder,
       with its age and size and the advice line, in the text and the JSON
2. [ ] Tests: a planted run folder with `previous\` shows, one without it does not

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
