---
size: S
origin: done/092-one-source-per-spec-the-router-and-the-generic-paths.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 101 — The update check stops per source at a rate limit, not as a whole

## Context

When one source's request budget runs out, the update check marks only that source's entries as
not checked and goes on asking the other sources; the stop line names the source whose limit it hit.
Why: the stop of the check applies to the whole run today, since GitHub was the one source with a budget;
with the Wago source (095) a second budget joins and one exhausted limit would silence the other source.
Done when: a fixture check over a GitHub entry and a Wago entry with one budget exhausted reports the one
source as not checked and the other as checked, in the text and the JSON, and `update --all` agrees.

Decided 2026-10-22: the first of four source follow-ups on one branch (101, 106, 104, 105), on the branch
of 094 and 095.

## Docs that move

- `docs/development.md`: the update check's budget stop, per source
- `README.md`: the rate-limit paragraph, if it says the check stops as a whole

## Not in this change

- The bulk Wago check (106); a budget for a source without one

## Steps

1. [ ] `update.rs`: the budget stop is kept per source kind; an exhausted budget marks that source's remaining
       entries not checked with the source's own stop line, the other sources go on
2. [ ] The text and the JSON name the source of each stop; the window's summary reads the same
3. [ ] Tests: a GitHub entry and a Wago entry with one budget exhausted (the stub's rate-limit answer of 095),
       `update --all` and the check, text and JSON

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
