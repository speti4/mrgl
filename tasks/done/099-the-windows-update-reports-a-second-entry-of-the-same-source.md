---
size: S
origin: done/089-update-one-reports-a-second-entry-of-the-same-source.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 099 — The window's update reports a second entry of the same source that has the pick's version

## Context

The window's update action (`crates/mrgl-app/src/actions.rs`) treats the "installed already" answer of
the one install path as its own outcome, naming the other entry of the same source, and counts it as
`installed_already` in the core summary of 089, as the command line does; the row's message says so.
Why: the window still counts that answer as updated and shows no line about the entry it found, the
behavior 089 fixed on the command line.
Done when: a window test over a sidecar with two entries from one source shows the line and counts the
addon as installed already, not updated, and the summary line reads as the command line's.

Decided 2026-10-22: the first of three follow-ups on one branch (099, 100, 102), in id order.

## Docs that move

- `docs/development.md`: the window's update action and its summary, if it lists the outcomes

## Not in this change

- The command line's update (089); the choice of the entry among two of one source (100)

## Steps

1. [x] `actions.rs`: the "installed already" answer of a second entry becomes its own outcome with the core's
       line, counted as `installed_already` in `SummaryOf::Update`; the row's message carries the line
2. [x] Tests: a window test over a sidecar with two entries from one source (the line, the counts, the summary
       line as the command line's)

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `crates/mrgl-core/src/install.rs`: `AlreadyInstalled` gains `is_entry_of`,
  `nothing_to_do_line` and `update_line`, the words of task 089 moved out of the command line: under the
  addon's own entry the nothing-to-do line (kind: nothing to do), under another entry of the same source the
  warning `<name> not updated: <entry>, a second entry of <source>, has <version> installed already in
  <flavor folder>; <name> stays at <version>`. `crates/mrgl-cli/src/main.rs` builds its `InstalledAlready`
  and the nothing-to-do summary of `add` from them; its output is unchanged.
- 2026-10-22 — `crates/mrgl-app/src/actions.rs`: a new phase `installed-already` (`Phase::InstalledAlready`)
  whose event carries the core's line in its kind; the sidecar's warnings of that answer join the answer's
  warnings; the run counts the addon as `installed_already` in `SummaryOf::Update`, not as updated. An addon
  found under its own entry reads from the disk again, as an updated one does; an addon another entry stood
  in for keeps the check's answer on its row.
- 2026-10-22 — UI: `ui/actions.js` gives the new phase a progress slot (`installed already`, full bar);
  `ui/mock-data.js` lists the phase in its event comment; `tests/ui/actions.test.js` asserts the slot.
- 2026-10-22 — test in `crates/mrgl-app/tests/update_actions.rs`: a flavor whose sidecar has `OldOneFolder`
  at 1.1.0 and `OneFolder` at 1.0.0, both `github:Example/Twice`, through the row update of `OneFolder`.
- 2026-10-22 — docs: `docs/development.md` (the window's phases and the shared line).

## Deviations from the plan

- The window's answer is its own phase, `installed-already`, rather than a message on the `updated` phase:
  the addon was not updated, and the row's progress slot says so.

## Verification

- 2026-10-22 — `tests/update_actions.rs`: the row update of `OneFolder` sends `checking`, `checked`,
  `updating`, then `installed-already` with the warning `OneFolder not updated: OldOneFolder, a second entry
  of github:Example/Twice, has 1.1.0 installed already in _forever_; OneFolder stays at 1.0.0`; the summary
  reads `nothing updated: 1 installed already`, the command line's summary of the same answer in task 089;
  the sidecar and `OneFolder`'s TOC are unchanged and the row keeps `update-available`. The code before the
  change sent that answer as `updated` with the progress line `OldOneFolder is at 1.1.0 already in
  _forever_` and counted it in `1 addon updated`.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green, the Node UI tests through `tests/ui_smoke.rs`), `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
