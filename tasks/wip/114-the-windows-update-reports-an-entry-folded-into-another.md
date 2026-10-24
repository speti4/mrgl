---
size: S
origin: done/108-update-all-over-two-entries-of-one-source-whose-first-install-takes-the-second-folder.md
created: 2026-10-22
started: 2026-10-24
plan: none
milestone: M2 Window MVP
---

# 114 — The window's update reports an entry folded into another during the run

## Context

The window's update action reads `Installed::folded` of the core (108) and reports an entry an earlier
install of the run folded into its addon with the core's line, as its own phase on the row, and counts it as
`folded` in the summary instead of passing `folded: 0`.
Why: 108 gave `update --all` the outcome and the line; the window still counts and says nothing about an
entry the run dropped.
Done when: a window test over the 108 fixture (two entries of one source, a zip with both folders) shows
the folded line on the second row and the summary counts one folded, and the sidecar holds one entry after.

Decided 2026-10-24: the first of three follow-ups on one branch (114, 115, 116), the window's update and add
actions.

## Docs that move

- `docs/development.md`: the window's update phases and summary counts, where it lists them

## Not in this change

- The command line's reporting of the case (108 did it)
- A distribution-off CurseForge entry's own state (115) and the window's zip in hand with a source (116)

## Steps

1. [ ] `crates/mrgl-app/src/actions.rs`: the update action keeps the entries the installs of the run folded,
       an update of one of them installs nothing and reports the core's `Folded::update_line` as its own phase
       on the row; the summary passes the count instead of `folded: 0`
2. [ ] The UI shows the phase on the row and the count in the summary, as the other phases and counts
3. [ ] Tests: a window test over the 108 fixture (`SeveralFolders` and `SeveralFolders_Options`, the zip with
       both folders): the folded line on the second row, one folded in the summary, one sidecar entry after;
       the Node tests of the UI modules for the new phase

## Log

- 2026-10-24 — promoted

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
