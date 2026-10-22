---
size: S
origin: done/076-the-checks-say-what-the-client-says.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 081 — The window's rows show the interface states `scan` and `list` mark

## Context

A row of the flavor list carries the addon's interface state, `interface mismatch` (015) or `out of date`
(076), from `interface::check_addon` and `interface::out_of_date_addon`, drawn as the command line marks
its lines, with the warning's words in the row's title.
Why: `scan` and `list` mark both states since 076 while the window shows neither, so an addon the client
lists as `Out of date` looks fine in the window.
Done when: a flavor list test over the `interface/` fixture sees the three rows (current, behind, another
expansion) with their states, and the mock data shows one of each.

## Docs that move

- `docs/development.md`: the flavor list's row fields (the interface state) in the UI section and the
  `flavors` command's answer
- `README.md`: the window's feature list, where the row marks are named

## Not in this change

- The command line's install summary (082); the window's update answer, which carries the core's checks
  already (079); a new interface rule in the core

## Steps

1. [ ] `crates/mrgl-app/src/list.rs`: each row carries its interface state (`interface mismatch` or
       `out of date`, from `interface::check_addon` and `interface::out_of_date_addon`) and the warning's
       words for the row's title
2. [ ] `ui/`: the row draws the state as the command line marks its lines, the warning's words in the title;
       the pure part with Node tests under `tests/ui/`; `mock-data.js` shows one row of each state
3. [ ] Tests: a flavor list test over the `interface/` fixture sees the three rows (current, behind, another
       expansion) with their states (`crates/mrgl-app/tests/flavor_list.rs`)
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
