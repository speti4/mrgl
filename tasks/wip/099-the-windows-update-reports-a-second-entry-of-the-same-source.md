---
size: S
origin: done/089-update-one-reports-a-second-entry-of-the-same-source.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 099 — The window's update reports a second entry of the same source that has the pick's version

## Context

The window's update action (`crates/mrgl-app/src/actions.rs`) treats the "installed already" answer of
the one install path as its own outcome, naming the other entry of the same source, and counts it as
`installed_already` in the core summary of 089, as the command line does; the row's message says so.
Why: the window still counts that answer as updated and shows no line about the entry it found, the
behavior 089 fixed on the command line.
Done when: a window test over a sidecar with two entries from one source shows the line and counts the
addon as installed already, not updated, and the summary line reads as the command line's.

Decided 2026-10-22: the first of three follow-ups on one branch (099, 100, 102), in id order.

## Docs that move

- `docs/development.md`: the window's update action and its summary, if it lists the outcomes

## Not in this change

- The command line's update (089); the choice of the entry among two of one source (100)

## Steps

1. [ ] `actions.rs`: the "installed already" answer of a second entry becomes its own outcome with the core's
       line, counted as `installed_already` in `SummaryOf::Update`; the row's message carries the line
2. [ ] Tests: a window test over a sidecar with two entries from one source (the line, the counts, the summary
       line as the command line's)

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
