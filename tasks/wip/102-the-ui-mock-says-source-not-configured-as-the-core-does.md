---
size: S
origin: done/092-one-source-per-spec-the-router-and-the-generic-paths.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
changelog: housekeeping
---

# 102 — The window's offline sketch says "source not configured" as the core does

## Context

The mock data of the window's offline sketch (`crates/mrgl-app/ui/mock-data.js`) writes the core's
line for a row whose source this build lacks, `source not configured: <spec> (this build has no <Name>
source)`, and its Node tests assert the same words.
Why: the sketch still says "a source the update check does not handle yet", the line the core wrote
before the router of 092.
Done when: the sketch's row text matches the core's line word for word and a Node test asserts it.

Decided 2026-10-22: the third of three follow-ups on one branch (099, 100, 102), in id order.

## Docs that move

- none

## Not in this change

- The window's rows themselves (they show the core's line already); any new sketch row

## Steps

1. [ ] `ui/mock-data.js`: the not-configured row's text is the core's line; the Node tests under
       `crates/mrgl-app/tests/ui/` assert it word for word

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
