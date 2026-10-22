---
size: S
origin: done/099-the-windows-update-reports-a-second-entry-of-the-same-source.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
changelog: housekeeping
---

# 107 — The window's row menu and the offline sketch follow the sources of this build

## Context

The row menu's "why no update" line (`ui/actions.js`) says the core's words for a source this build
lacks or whose key is not set (`source not configured: …`, `needs <KEY NAME>`) instead of "the update check
does not ask this source yet", and the offline sketch (`ui/mock-data.js`) stops assuming a GitHub-only
build: its WoWInterface and Wago rows carry the answers the sources of 094 and 095 give; the Node tests
assert both.
Why: 092 to 095 made every source askable, and the sketch and the menu still describe the GitHub-only build.
Done when: the menu line and the sketch rows match the core's words for a WoWInterface, a Wago and a
not-configured row, asserted by the Node tests.

Decided 2026-10-22: the second of two follow-ups on one branch (108, 107).

## Docs that move

- `docs/development.md`: the offline sketch's description, if it says the sketch is GitHub-only

## Not in this change

- The window's rows themselves (they show the core's words); the pending WoWInterface state (104)

## Steps

1. [ ] `ui/actions.js`: the "why no update" line takes the core's words from the row's check answer
2. [ ] `ui/mock-data.js`: WoWInterface and Wago rows with the answers of 094 and 095, a not-configured row
       with the core's line; the Node tests under `crates/mrgl-app/tests/ui/` assert the three

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
