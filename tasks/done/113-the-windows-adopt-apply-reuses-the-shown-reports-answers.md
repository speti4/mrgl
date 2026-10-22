---
size: S
origin: done/109-the-windows-adopt-view.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 113 — The window's adopt apply reuses the shown report's answers

## Context

The toolbox's adopt apply writes the sidecar entries from the report the card shows instead of asking
every source again, and the report after the write is built from the written sidecar and the same answers;
a report older than the sources' cache rule is refreshed first.
Why: 109 asks every source twice more after the report the user saw, which spends requests against GitHub's
unauthenticated limit for nothing new.
Done when: a window test over the adopt fixture counts the stub's requests: one round for the report, none
for the apply and the report after it, and the written sidecar equals the one of 109's test.

Decided 2026-10-22: the second of two adopt-card follow-ups on one branch (112, 113), after 112.

## Docs that move

- `docs/development.md`: the toolbox's `adopt` and `adopt_apply` commands (the report carried to the apply)

## Not in this change

- The command line's `adopt --apply` (it reads once and writes, no shown report to reuse)

## Steps

1. [x] `adopt.rs`: the apply takes a report value (the sources' answers) and writes from it; the report after
       the write is built from the written sidecar and the same answers; an age rule refreshes a stale report
2. [x] `commands.rs` and the UI: the card hands the shown report to `adopt_apply`
3. [x] Tests: a window test counts the stub's requests (one round for the report, none for the apply and the
       report after), the written sidecar equals 109's

## Log

- 2026-10-22 — promoted
- 2026-10-22 — core: `adopt::Answers` (the flavor id, `asked_at` when the asking began, `offers` per
  canonical spec as `adopt::Answer` with `build`, `why` and `missing`, the clause of what the run lacked
  then, and `folders` per spec, `null` for none), serialized both ways; `adopt::Answering`, a `Source` that
  answers `offers` and `folders` from the answers it holds and asks the source under it for the rest,
  holding what it gets (a spec whose `missing` changed since, a key set or removed, is asked again);
  `adopt::report_answering(scan, state, sources, has_key, answers)` returns the report and the answers.
  `Answers::stand_for(flavor)`: the flavor's own answers, asked less than `adopt::ANSWERS_MAX_AGE_SECONDS`
  (3600) ago; a time ahead of the clock or one that does not read does not stand. The hour is the shortest
  cache rule of the sources: WoWInterface's project answer (`Cache-Control: max-age=3600`); its file list
  and Wago's slug lists keep a day, GitHub asks again with its `ETag`. `sources::Offered` deserializes.
- 2026-10-22 — window: `AdoptView` carries `answers`; `toolbox::adopt_report` builds the report through
  fresh answers; `toolbox::adopt_apply(root, flavor, AdoptRequest { alongside, answers }, …)` takes the
  shown report's answers when they stand for the flavor (else asks every source again, as 109 did), builds
  the report over the folders and the sidecar as they are now and those answers, applies, and builds the
  report after the write over the written sidecar and the same answers, which its view carries for the next
  apply. The `adopt_apply` command takes `answers` (optional); `data.js` sends `answers` (or `null`),
  `main.js` the `adopt.answers` of the report the card shows. The sketch's report carries made-up answers,
  and its apply hands the shown ones back.
- 2026-10-22 — tests: `crates/mrgl-app/tests/adopt.rs`
  `the_apply_writes_from_the_shown_reports_answers_and_asks_no_source` (the report over the folder stub
  with its request log, the answers through JSON as the page sends them, the apply over a new router on
  the same stub: no request for the apply and the report after it, the six entries and the four remaining
  rows, the same answers back, and the sidecar equal to the one of the apply that asks every source again,
  the times of the entries left out) and `old_answers_or_another_flavors_are_asked_again`; the contract's
  keys with `answers`; the core's `answers_stand_for_their_flavor_for_an_hour`. Node: `data.test.js` (the
  answers sent, `null` without them), `mock-data.test.js` (the answers' keys in every flavor, the shown
  answers back after the apply).
- 2026-10-22 — docs: `docs/development.md`, the Adopt section (the answers, `Answering`, the hour), the
  `adopt` and `adopt_apply` commands, the `Data.adoptApply` contract and the window tests.

## Deviations from the plan

- The apply does not write the shown report's rows as they are: it builds the report again over the
  folders and the sidecar as they are now and the shown report's answers (`adopt::Answering`), and
  `adopt::apply` writes from that report, unchanged. No source is asked again, and a folder or an entry
  that changed since the report is seen, which was 109's reason to ask again; a spec the answers lack (a
  folder that arrived since) is asked.
- `toolbox::adopt_apply` takes the confirmation and the answers as one `AdoptRequest`, which keeps the
  function within clippy's argument count.

## Verification

- 2026-10-22 — `cargo test --locked`: every suite green, among them the `mrgl-core` unit tests (with
  `answers_stand_for_their_flavor_for_an_hour`), `crates/mrgl-core/tests/adopt.rs`,
  `crates/mrgl-cli/tests/adopt.rs` (the command line unchanged), `crates/mrgl-app/tests/adopt.rs` (8, the
  request count: one round for the report, none for the apply and the report after it; the sidecar equal to
  109's) and `ui_smoke.rs`, whose `node --test` runs the 87 Node tests of `tests/ui/`. `cargo fmt
  --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check --all-targets
  --locked` and `python -I scripts/changelog.py --check` green; `tasks_check` without `write`: only the
  stale index, which the main session regenerates. No manual round on the window (one manual round at a
  time across the sessions).

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
