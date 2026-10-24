---
size: S
origin: done/108-update-all-over-two-entries-of-one-source-whose-first-install-takes-the-second-folder.md
created: 2026-10-22
started: 2026-10-24
closed: 2026-10-24
plan: none
milestone: M2 Window MVP
---

# 114 — The window's update reports an entry folded into another during the run

## Context

The window's update action reads `Installed::folded` of the core (108) and reports an entry an earlier
install of the run folded into its addon with the core's line, as its own phase on the row, and counts it as
`folded` in the summary instead of passing `folded: 0`.
Why: 108 gave `update --all` the outcome and the line; the window still counts and says nothing about an
entry the run dropped.
Done when: a window test over the 108 fixture (two entries of one source, a zip with both folders) shows
the folded line on the second row and the summary counts one folded, and the sidecar holds one entry after.

Decided 2026-10-24: the first of three follow-ups on one branch (114, 115, 116), the window's update and add
actions.

## Docs that move

- `docs/development.md`: the window's update phases and summary counts, where it lists them

## Not in this change

- The command line's reporting of the case (108 did it)
- A distribution-off CurseForge entry's own state (115) and the window's zip in hand with a source (116)

## Steps

1. [x] `crates/mrgl-app/src/actions.rs`: the update action keeps the entries the installs of the run folded,
       an update of one of them installs nothing and reports the core's `Folded::update_line` as its own phase
       on the row; the summary passes the count instead of `folded: 0`
2. [x] The UI shows the phase on the row and the count in the summary, as the other phases and counts
3. [x] Tests: a window test over the 108 fixture (`SeveralFolders` and `SeveralFolders_Options`, the zip with
       both folders): the folded line on the second row, one folded in the summary, one sidecar entry after;
       the Node tests of the UI modules for the new phase

## Log

- 2026-10-24 — promoted
- 2026-10-24 — `crates/mrgl-app/src/actions.rs`: the update run keeps the entries the installs of the run
  dropped (`Installed::folded`, task 108); an update of one of them installs nothing, asks nothing more and
  sends the new phase `folded` (`Phase::Folded`) with the core's `Folded::update_line` as the event's
  message, on the dropped entry's row. The summary passes their count as `SummaryOf::Update::folded`
  (`1 addon updated, 1 folded into another entry`), and the row of a folded entry reads from the disk again,
  as the rows of the updated addons do, instead of keeping the check's answer of an entry that is gone.
- 2026-10-24 — the UI: `actions.js`'s `STEPS` has `folded` (`folded into another entry`, the bar full), so
  the row's progress slot shows the phase as it shows `installed already`; the event's line goes through
  the status bar as every event's line does, and the summary is the core's message. The progress event
  contract in `mock-data.js` names the phase.
- 2026-10-24 — tests: `tests/update_actions.rs`
  `update_all_says_an_entry_an_earlier_install_folded_into_its_addon` (`SeveralFolders` and
  `SeveralFolders_Options`, both `github:Example/Folded` at 2.0.0 with their own folders, the release 2.1.0
  the fixture zip `several-folders-2.1.0.zip` with both folders); the Node test of the progress slot.
- 2026-10-24 — docs: `docs/development.md` (the update run's phases with `folded` and its summary count,
  and the window's side of the 108 paragraph).

## Deviations from the plan

- none

## Verification

- 2026-10-24 — `tests/update_actions.rs`: "update all" over the two entries sends `checking`/`checked` for
  both, `updating`, `downloading`, `installing` and `updated` for `SeveralFolders`, then `folded` on the row
  `SeveralFolders_Options` (index 2 of 2, kind `success`) with `SeveralFolders_Options folded into
  SeveralFolders: SeveralFolders 2.1.0 ships its folder SeveralFolders_Options in _forever_,
  SeveralFolders_Options has no entry of its own now`; the stub source is asked twice (the check only), the
  summary is `1 addon updated, 1 folded into another entry` (a success), the warning `SeveralFolders_Options
  is no longer managed: SeveralFolders replaced every folder of it` is among the messages, and the sidecar
  holds the one entry `SeveralFolders` at 2.1.0 with both folders; every row is managed with no update.
- 2026-10-24 — `node --test crates/mrgl-app/tests/ui/*.test.js`: 88 tests green. `cargo fmt --check`,
  `cargo clippy --all-targets --locked -- -D warnings` and `cargo test --locked` green.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
