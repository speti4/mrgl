---
size: S
origin: done/109-the-windows-adopt-view.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 113 — The window's adopt apply reuses the shown report's answers

## Context

The toolbox's adopt apply writes the sidecar entries from the report the card shows instead of asking
every source again, and the report after the write is built from the written sidecar and the same answers;
a report older than the sources' cache rule is refreshed first.
Why: 109 asks every source twice more after the report the user saw, which spends requests against GitHub's
unauthenticated limit for nothing new.
Done when: a window test over the adopt fixture counts the stub's requests: one round for the report, none
for the apply and the report after it, and the written sidecar equals the one of 109's test.

Decided 2026-10-22: the second of two adopt-card follow-ups on one branch (112, 113), after 112.

## Docs that move

- `docs/development.md`: the toolbox's `adopt` and `adopt_apply` commands (the report carried to the apply)

## Not in this change

- The command line's `adopt --apply` (it reads once and writes, no shown report to reuse)

## Steps

1. [ ] `adopt.rs`: the apply takes a report value (the sources' answers) and writes from it; the report after
       the write is built from the written sidecar and the same answers; an age rule refreshes a stale report
2. [ ] `commands.rs` and the UI: the card hands the shown report to `adopt_apply`
3. [ ] Tests: a window test counts the stub's requests (one round for the report, none for the apply and the
       report after), the written sidecar equals 109's

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
