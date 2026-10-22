---
size: S
origin: done/058-forever-notes-for-the-partly-restricted-addons.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 070 — The Forever notes schema check allows one day of slack on `verified`

## Context

The schema test of `data/forever-notes.json` (011) accepts a `verified` date up to one day ahead of today
in UTC, and `CONTRIBUTING.md` ("Forever notes") points to the rule for adding a note in
`docs/development.md`. East of UTC, a note dated by the local day fails as "in the future" between local
and UTC midnight.
Done when: a note dated tomorrow in UTC passes, one dated two days ahead fails, and CONTRIBUTING links the
rule.

Decided 2026-10-22: second of the chain 069 → 070 on one branch.

## Docs that move

- `docs/development.md` ("Forever notes"): the rule for adding a note states the date slack.
- `CONTRIBUTING.md` ("Forever notes"): links the rule (the link is there; the sentence names the slack).

## Not in this change

- The stale-note age (`STALE_AFTER_DAYS`); the notes' content (058, rechecked after the launch in 008).

## Steps

1. [ ] `forever.rs`: `CuratedNotes::problems(today)` accepts `verified` up to `today + 1`; the problem text
       of a later date says so
2. [ ] Tests: a note dated tomorrow passes, one dated two days ahead fails, the compiled-in file still holds
3. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; second of the chain 069 → 070

## Deviations from the plan

- none yet

## Verification

- (dated, concrete: what was run, what it showed)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
