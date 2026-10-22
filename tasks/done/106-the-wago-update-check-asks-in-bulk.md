---
size: S
origin: done/095-the-wago-source.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M4 Sources
---

# 106 — The Wago update check asks for its addons in bulk, one request per game version

## Context

The update check gathers the Wago entries per game version and asks `POST /addons/_recents` once
for each, through the `Http::post` of 092, instead of one request per addon; the cache and the per-addon
answer shape stay the same, and a single `add` or `update <name>` still asks for the one addon.
Why: one request per addon against a limit of 60 runs out on a folder of a few dozen Wago addons; the
bulk endpoint makes the check one request per game version.
Done when: a fixture check over several Wago entries makes one stubbed POST per game version, the answers
land in the same cache files, and the budget test of 095 still stops before the limit.

Decided 2026-10-22: the second of four source follow-ups on one branch (101, 106, 104, 105), after 101.

## Docs that move

- `docs/development.md`: the Wago source's bulk check and its stub (a POST answer file)

## Not in this change

- A bulk check for the other sources; the per-source stop itself (101)

## Steps

1. [x] `sources/wago.rs`: a bulk lookup of several ids per game version through `Http::post`, parsed into
       the same per-addon answer and written into the same cache files
2. [x] `update.rs`: the check gathers the Wago entries per game version and asks once; `add` and
       `update <name>` keep the single request
3. [x] Tests: a stubbed POST answer per game version, one request per version over several entries, the
       cache files, the budget stop of 095 unchanged

## Log

- 2026-10-22 — promoted
- 2026-10-22 — the trait: `Source::prefetch(specs, target)` (ask about several specs at once before the
  check asks them one by one; default nothing) and `Source::holds(spec, target)` (whether resolving the spec
  sends no request; default no). The router hands each source the specs of its kind that it can ask.
- 2026-10-22 — `sources::wago`: `Wago::recents(ids, game_version)` sends `POST
  https://addons.wago.io/api/external/addons/_recents?game_version=<v>` with `{"addons": [<id>, …]}`,
  `Content-Type: application/json` and the key (none without it), counted against the budget like any
  request to Wago's host. `parse_recents` reads the answer as the addons under `addons` (or `data`, or the
  answer itself), an object by id or a list with `id`s, and keeps only the ids asked; each answer that reads
  as an addon is held for the run and written into `cache/wago/<id>.json` under its game version, the same
  entry a single answer writes (`Wago::store`, shared with `Wago::addon`). `Wago::addon` answers a held
  addon without a request. The source's `prefetch` sends one bulk request for the target's game version
  when it has the key and a request left; a failed bulk request is no error, each addon is then asked alone.
- 2026-10-22 — `update::check_until`: a check of every addon (not `update <name>`) prefetches the specs it
  will ask with the flavor's target, then reads the budget only for an entry the source does not hold, and
  counts a stop's addons left among those alone. `add`, `import` and `update <name>` keep the single
  request.
- 2026-10-22 — fixtures: `addons.wago.io/api/external/addons/_recents@game_version=forever.json`
  (`MrglWg01`, `MrglWg03`) and `…=retail.json` (`MrglWg01` to `MrglWg03`), built from the single answers;
  `MrglWg05` stays out of the Forever bulk answer, so its single answer still uses the limit up.
- 2026-10-22 — tests: `tests/wago_source.rs`
  `the_update_check_asks_wago_once_per_game_version_and_caches_each_answer` (a Forever and a retail check
  over three Wago entries: one POST each with the ids, the key and the content type, a single request only
  for the addon the bulk answer lacks, the states, the cache files with both game versions),
  `one_addons_check_keeps_the_single_request_and_a_used_up_budget_sends_no_bulk` (`update <name>` sends the
  one GET; with no request left no POST goes out and the check stops before the limit) and
  `a_bulk_answer_is_held_for_the_run_and_answers_without_a_request`; core unit
  `a_bulk_answer_reads_by_id_in_either_shape_and_keeps_the_ids_asked`; `tests/cli.rs`
  `update_all_takes_the_wago_answers_from_one_bulk_request` (a stub without the single answers still
  answers the check, the cache files hold the answers, `update One` asks the single address).
- 2026-10-22 — docs: `docs/development.md` (the layout's Wago source, the bulk check and its stub files in
  the Wago paragraph under Fixtures), `tests/fixtures/README.md` (the bulk answers), `README.md` (one request
  per flavor's Wago addons).

## Deviations from the plan

- The budget test of 095 (`import_takes_a_wago_entry_and_update_stops_before_wagos_rate_limit`) gained a
  fourth entry: with the bulk answer holding `MrglWg01` and `MrglWg03`, those two need no request after the
  limit is used up, so the stop now leaves `MrglWg02`, which the bulk answer lacks, and the test checks that
  the held addons are answered and the one left is not asked.
- The check gathers the specs once per run with the flavor's target rather than grouping them per game
  version itself: a flavor has one game version, so the source sends one request per check, and the game
  version stays the source's own business.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (`tests/cli.rs` 149, `tests/wago_source.rs` 10, core
  unit 233, the other targets unchanged). `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D
  warnings`, `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check` green.
  The external API was not asked; the bulk answers are stubbed in the field shape of the single answers.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
