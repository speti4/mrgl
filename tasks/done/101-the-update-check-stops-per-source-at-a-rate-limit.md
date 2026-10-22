---
size: S
origin: done/092-one-source-per-spec-the-router-and-the-generic-paths.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
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

1. [x] `update.rs`: the budget stop is kept per source kind; an exhausted budget marks that source's remaining
       entries not checked with the source's own stop line, the other sources go on
2. [x] The text and the JSON name the source of each stop; the window's summary reads the same
3. [x] Tests: a GitHub entry and a Wago entry with one budget exhausted (the stub's rate-limit answer of 095),
       `update --all` and the check, text and JSON

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `update::check_until` keeps the stops per `SourceKind`: before an entry that needs a request
  it reads the budget of the entry's source (`Source::request_budget_for`), and once that budget has
  nothing left it stops asking that source alone. The source's remaining entries are `rate-limited` with
  `not asked before <Host>'s rate limit`, `CheckProgress::Stopped` goes out once per source, and the entries
  of the other sources are still asked. `BudgetStop` gained `source` (`github`, `wago` in the JSON); its
  line now names the source as the one stopped, `stopped asking <Host> before its rate limit of <n>
  requests<window>: <n> addons not checked, …`, since the check itself goes on.
- 2026-10-22 — `UpdateCheck::stopped` became a `Vec<BudgetStop>`, one per source in the order they stopped;
  `mrgl update --json`'s `stopped` is that list (each stop with its `message`), empty when nothing stopped.
  `UpdateCheck::summary`, which the command line and the window share, counts the `rate-limited` addons per
  source: `1 not checked (Wago's rate limit)`, `2 not checked (GitHub's rate limit)`.
- 2026-10-22 — tests: `tests/cli.rs` `update_stops_asking_wago_at_its_rate_limit_and_goes_on_asking_github`
  (one stub of the GitHub and the source fixtures: `wago:MrglWg05` answers with no request left, the next
  Wago entry is not checked, the GitHub entry after it is checked; the text, the JSON and `update --all
  --yes`, which installs the GitHub update and ends with exit code 0); the window's
  `tests/update_actions.rs` `the_budget_stop_of_one_source_leaves_the_other_source_asked` (a GitHub stop,
  one event, a Wago addon after it checked, the summary per source); core unit
  `the_summary_counts_the_addons_left_before_a_rate_limit_per_source` and the stop line's `source` in the
  JSON.
- 2026-10-22 — docs: `docs/development.md` (the update check's budget per source, the `Stopped` event once
  per source, the JSON's `stopped` list, the window's `stopped` phase, the Wago budget's stop), `README.md`
  (the GitHub rate-limit sentence of the update feature, the Wago paragraph).

## Deviations from the plan

- The stop line and the not-checked reason changed their words for every source, GitHub included: `stopped
  the check before GitHub's rate limit …` is now `stopped asking GitHub before its rate limit …` and the
  reason `the check stopped before GitHub's rate limit` is `not asked before GitHub's rate limit`, since the
  check no longer stops as a whole. The tests of 068 and 095 that pinned the old words
  (`update_stops_before_the_rate_limit_with_the_reset_time_and_counts_the_rest`,
  `import_takes_a_wago_entry_and_update_stops_before_wagos_rate_limit`, the window's
  `the_budget_stop_is_one_event_and_the_summary_counts_the_rest`, the core's line tests) now expect the new
  ones, and the summary's `2 not checked (rate limit)` is `2 not checked (GitHub's rate limit)`.
- The JSON's `stopped` changed shape, from one object or `null` to a list of stops (empty when none), so a
  check with two stops can carry both; `update_json_is_one_document_with_every_state_and_count` expects `[]`.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (`tests/cli.rs` 148, core unit 232,
  `tests/update_actions.rs` 11, the other targets unchanged). `cargo fmt --check`, `cargo clippy
  --all-targets --locked -- -D warnings`, `cargo +1.90 check --all-targets --locked` and `python -I
  scripts/changelog.py --check` green. No real game folder and no network: the sources answer from the
  fixture stub.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
