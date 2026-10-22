---
size: S
origin: done/109-the-windows-adopt-view.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
changelog: housekeeping
---

# 112 — The toolbox's adopt lines in the window's words, without the command line's flags

## Context

The core's adopt summary, marker line and refusal take a caller's wording, as `CleanupBy` does for the
SavedVariables summary (086), so the toolbox's adopt card says "the Adopt button adopts N of them" and names
its confirmation instead of `adopt --apply` and `--alongside-curseforge`; the command line keeps its flags.
Why: 109 shows the core's lines as they are, and three of them name command-line flags the window has not.
Done when: the card's three lines name no flag, the command line's are unchanged, and the Rust and Node tests
assert both wordings.

Decided 2026-10-22: the first of two adopt-card follow-ups on one branch (112, 113), on the branch of 110
and 109.

## Docs that move

- `docs/development.md`: the adopt lines' caller wording, next to `CleanupBy`

## Not in this change

- Any new adopt line; the apply's requests (113)

## Steps

1. [ ] `adopt.rs`: the summary line, the marker line and the refusal take a wording value (the command line's
       flags, the window's button and confirmation), as `CleanupBy` does
2. [ ] The command line passes its wording (unchanged output); the toolbox passes the window's
3. [ ] Tests: the CLI tests unchanged, the window tests and the Node tests assert the window's three lines

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
