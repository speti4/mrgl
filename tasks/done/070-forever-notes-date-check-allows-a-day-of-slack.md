---
size: S
origin: done/058-forever-notes-for-the-partly-restricted-addons.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
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

1. [x] `forever.rs`: `CuratedNotes::problems(today)` accepts `verified` up to `today + 1`; the problem text
       of a later date says so
2. [x] Tests: a note dated tomorrow passes, one dated two days ahead fails, the compiled-in file still holds
3. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; second of the chain 069 → 070
- 2026-10-22 — `forever.rs`: the slack is the constant `VERIFIED_SLACK_DAYS` (1), next to
  `STALE_AFTER_DAYS`; `CuratedNotes::problems(today)` takes a `verified` day up to `today +
  VERIFIED_SLACK_DAYS`, for a kind and for a note alike, and a later date reads `verified <date> is in the
  future, more than 1 day after today in UTC`. The stale rule is unchanged (a date a day ahead is not stale).
- 2026-10-22 — tests: `a_verified_date_may_be_one_day_after_today_in_utc` over the synthetic file with every
  date set to 2026-10-21 and to 2026-10-22 (no problem against the day 2026-10-21) and to 2026-10-23 (exactly
  the three problems of the kind and the two notes, with the new text); `the_schema_check_names_every_problem`
  and `the_compiled_in_file_holds_to_the_schema` unchanged and green.
- 2026-10-22 — `docs/development.md` ("Forever notes"): the rule for adding a note says the date is the local
  day of the check, compared with today in UTC with one day of slack (`forever::VERIFIED_SLACK_DAYS`), and
  that two days ahead fails. `CONTRIBUTING.md` ("Forever notes"): the sentence names the slack and links the
  rule (`docs/development.md#forever-notes`, the anchor of the "Forever notes" heading).

## Deviations from the plan

- none

## Verification

- 2026-10-22 — `cargo test --locked -p mrgl-core --lib forever`: the seven Forever tests pass, the new one
  among them; with `VERIFIED_SLACK_DAYS` set to 0 for one run, the new test failed on the date 2026-10-22
  (`verified 2026-10-22 is in the future, more than 0 day after today in UTC`), so it covers the slack.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (core unit 175, `tests/cli.rs` 115, every other suite green) and `cargo +1.90 check
  --all-targets --locked` green; `python -I scripts/changelog.py --check` green after the regeneration.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
