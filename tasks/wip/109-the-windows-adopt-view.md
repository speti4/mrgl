---
size: M
origin: done/097-adopt-the-report-of-the-sources-of-an-addons-folder.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 109 — The window's adopt view: the report and `--apply` in the toolbox

## Context

The toolbox shows the adopt report of the chosen flavor (097: per unmanaged addon its sources and
whether each offers a build, the summary counts, the `.curseclient` marker) and applies it with one click
(098: the same sidecar write, the `--alongside-curseforge` confirmation as a dialog, the warning after),
over the core's lines and counts, with the same Rust and Node tests as the toolbox's other views.
Why: adopt is the road to CurseForge independence, and the window is where most users will take it.
Done when: a window test over the adopt fixture shows the report's rows and counts, the apply writes the
sidecar the command line's test accepts, and the marker guard holds with and without the confirmation.

Decided 2026-10-22: the second of two window follow-ups of adopt on one branch (110, 109).

## Docs that move

- `docs/development.md`: the toolbox's adopt view and its commands
- `README.md`: the window's adopt paragraph, next to the command line's

## Not in this change

- A new core line or count (the view shows the core's); the CurseForge source (096)

## Steps

1. [ ] `toolbox.rs` and `commands.rs`: the adopt report of a flavor as a toolbox view over `adopt.rs`'s
       report, the apply command over the same write, the marker confirmation and the warning after
2. [ ] The UI: the adopt section of the toolbox (rows, counts, the apply button, the confirmation dialog), the
       sketch's mock data for it
3. [ ] Tests: the window tests over the adopt fixture (the rows and counts, the apply's sidecar, the guard with
       and without the confirmation), the Node tests of the section

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
