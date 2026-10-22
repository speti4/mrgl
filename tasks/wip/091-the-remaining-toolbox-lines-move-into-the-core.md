---
size: S
origin: done/086-one-set-and-savedvars-summary-in-the-core.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M3 Toolbox
changelog: housekeeping
---

# 091 — The remaining toolbox lines move into the core: the set list note, the not-installed warning and the two-account error's words

## Context

The set list note (`2 sets of _forever_ in …`) and the apply's not-installed warning move into the
core as the summaries of 086 did, with the command line and the window's toolbox (085) both printing them
from there; and the core's two-account error names the account to pick without the command line's
`--account` flag, so the window can show the core's words.
Why: 086 moved the three summaries and left these two lines in both callers, and the two-account error ends
with a flag the window never has.
Done when: one function per line in the core with both callers, the CLI and the window tests assert the same
text, and the two-account error reads the same from both without naming a flag.

Decided 2026-10-22: a branch of its own, in parallel with the wave-7 follow-ups of the command line (088 to 090).

## Docs that move

- `docs/development.md`: the toolbox summaries in the core, if it lists them

## Not in this change

- The toolbox features themselves (085, 086); any new toolbox line

## Steps

1. [ ] The core: one function for the set list note and one for the not-installed warning, next to the
       summaries of 086; the two-account error names the account to pick without a flag
2. [ ] The command line and the window's toolbox print the core's lines; the CLI keeps its flag hint next to
       the core's words where it needs one
3. [ ] Tests: the CLI tests and the window tests (Rust and the Node UI tests) assert the same text

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
