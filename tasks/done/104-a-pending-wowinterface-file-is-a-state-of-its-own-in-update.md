---
size: S
origin: done/094-the-wowinterface-source.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M4 Sources
---

# 104 — A WoWInterface file waiting for approval is a state of its own in the update check

## Context

A WoWInterface project whose newest file waits for the site's approval (`UIPending`) gets its own
state in the update check and the window's row (a pending release, nothing to do yet), counted apart from
the failed checks, so `update --all` ends with exit code 0 while the site approves it.
Why: 094 reports a pending file as a failed check, which turns a daily run red for a day or two that is the
site's, not the user's.
Done when: the pending fixture (id 1003) ends an `update --all` with exit code 0 and its own line and JSON
state, the window's row shows it, and a failed check still ends with 1.

Decided 2026-10-22: the third of four source follow-ups on one branch (101, 106, 104, 105), after 106.

## Docs that move

- `docs/development.md`: the update states (the pending release), the window's state list

## Not in this change

- Installing the previous, approved file instead (the API offers one file per project)

## Steps

1. [x] `update.rs`: a pending release is its own state (`Error::ReleasePending` of 094 maps onto it), counted
       apart from the failed checks, exit code 0 for a run with pending entries only
2. [x] The text (through the voice), the JSON state and the window's row and state list carry it
3. [x] Tests: the pending fixture under `update --all` (exit 0, the line, the JSON), a window row, a failed
       check still exits 1

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `UpdateState::Pending` (`pending` in the JSON, the label `pending approval`), listed in
  `UpdateState::ALL` after `ignored`: the check maps `Error::ReleasePending` of an addon that is not pinned
  onto it (a pinned one stays `pinned`, as for any error), with the detail `the newest file waits for
  approval on <host>; nothing to install until it is approved`, so the data line reads `MrglPending  1.0.0
  pending approval: the newest file waits for approval on WoWInterface; nothing to install until it is
  approved`. The summary counts it apart (`1 pending approval`) and does not make it a warning; `mrgl
  update` counts only `failed` checks and failed installs towards exit code 1, so a run with pending
  entries alone ends with 0. The JSON's `counts` gained `pending`.
- 2026-10-22 — the window: the row's `check` carries the state and its label from the core, and the
  offline list gives it again from the WoWInterface source's cached answer (no entry in `last-check.json`
  needed); the note under a row's version names it (`actions.js`'s `NOTED` gained `pending`), and the
  offline sketch's labels and summary order (`mock-data.js`) know it.
- 2026-10-22 — tests: `tests/cli.rs` `import_and_update_take_a_wowi_entry` (the pending fixture `1003`
  under `update --all --dry-run`: exit code 0, its line, the summary, the JSON state, detail and count;
  pending and an unconfigured Wago entry end with 0; with the key the Wago entry's 401 is a failed check
  that ends with 1 next to the pending one) and `update_json_is_one_document_with_every_state_and_count`
  (the `pending` count); the window's `tests/flavor_list.rs`
  `a_row_of_a_file_waiting_for_approval_shows_the_pending_state`; the Node test of `noteOf`; core unit
  `a_pending_file_is_counted_apart_and_needs_no_look` and the pending line in
  `every_state_has_its_line`.
- 2026-10-22 — docs: `docs/development.md` (the update states, the WoWInterface paragraph under Fixtures,
  the states the window's row note names), `README.md` (the states of the update check, the WoWInterface
  paragraph).

## Deviations from the plan

- The failed check that still ends with exit code 1 is a Wago entry answered 401 (`wago:MrglWg04` with the
  made-up key), since no WoWInterface fixture fails to answer; the pending entry next to it keeps its own
  line.
- `import` still reports a pending file as failed and `add` still fails with it: the task names the update
  check, and both of those were asked to install the file.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (`tests/cli.rs` 149, core unit 234,
  `tests/flavor_list.rs` 18, the UI's Node tests 80 through `tests/ui_smoke.rs`, the other targets
  unchanged). `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
