---
size: S
origin: done/076-the-checks-say-what-the-client-says.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 081 — The window's rows show the interface states `scan` and `list` mark

## Context

A row of the flavor list carries the addon's interface state, `interface mismatch` (015) or `out of date`
(076), from `interface::check_addon` and `interface::out_of_date_addon`, drawn as the command line marks
its lines, with the warning's words in the row's title.
Why: `scan` and `list` mark both states since 076 while the window shows neither, so an addon the client
lists as `Out of date` looks fine in the window.
Done when: a flavor list test over the `interface/` fixture sees the three rows (current, behind, another
expansion) with their states, and the mock data shows one of each.

## Docs that move

- `docs/development.md`: the flavor list's row fields (the interface state) in the UI section and the
  `flavors` command's answer
- `README.md`: the window's feature list, where the row marks are named

## Not in this change

- The command line's install summary (082); the window's update answer, which carries the core's checks
  already (079); a new interface rule in the core

## Steps

1. [x] `crates/mrgl-app/src/list.rs`: each row carries its interface state (`interface mismatch` or
       `out of date`, from `interface::check_addon` and `interface::out_of_date_addon`) and the warning's
       words for the row's title, as `interface_check: { state, label, line } | null`; the marks' words are
       one constant each in the core (`InterfaceMismatch::LABEL`, `InterfaceOutOfDate::LABEL`), which the
       command line's line mark uses too
2. [x] `ui/`: the row draws the state as the command line marks its lines, the warning's words in the title;
       the pure part with Node tests under `tests/ui/`; `mock-data.js` shows one row of each state
3. [x] Tests: a flavor list test over the `interface/` fixture sees the three rows (current, behind, another
       expansion) with their states (`crates/mrgl-app/tests/flavor_list.rs`)
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — core: `InterfaceMismatch::LABEL` (`interface mismatch`) and `InterfaceOutOfDate::LABEL`
  (`out of date`), the marks of an addon's line; `with_interface_note` of the command line appends them,
  so `scan` and `list` print the same words as before. No interface rule changed.
- 2026-10-22 — `src/list.rs`: a row gains `interface_check`, an `InterfaceFact` (`state` `mismatch` or
  `out-of-date`, `label` the mark, `line` the warning's plain line, `InterfaceMismatch::line` or
  `InterfaceOutOfDate::line`) built by `InterfaceFact::of` from `interface::check_addon`, then
  `interface::out_of_date_addon`, against the scan's build; `null` for a current TOC, an unknown build, a TOC
  without `## Interface` or a folder the flavor loads nothing from. Every answer that carries a flavor (the
  list, a run, a mark) carries it, since all of them go through `flavor_view`.
- 2026-10-22 — UI: `state.js` gains `INTERFACE_STATES`, `interfaceMark(addon)` (class, label, title, or
  null) and `interfaceLabelOf(state)`; `main.js` draws the mark as a pill of its own under the state's pill
  (`.interface-mark` in the row template of `index.html`), the warning's line in its title, and the legend
  lists both marks after the states; `style.css` has a token per mark (`--interface-mismatch` on `--bad`,
  `--interface-out-of-date` on `--warn`), light and dark through the tokens they point to.
  `mock-data.js`: the contract names `interface_check`; retail's `MyUnitFrames` is out of date (120100) and
  `OldMinimap` an interface mismatch (110207), each line in the core's words for retail's build.
- 2026-10-22 — tests: `the_rows_carry_the_interface_states_scan_and_list_mark` in `tests/flavor_list.rs`
  reads the `interface/` fixture in place with a temporary home folder: `Current` has no mark, `Behind` is
  `out-of-date` and `Elsewhere` `mismatch`, with their lines and their JSON, and nothing is written next to
  the fixture; the contract test's row keys gain `interface_check`, and no row of its made-up game has one.
  Node: `state.test.js` (the mark of each state, the table's words without a label, nothing for an unknown
  state) and `mock-data.test.js` (the key, one row of each state, each line naming its flavor's build).
- 2026-10-22 — docs: `docs/development.md` (the row's mark in the UI section, the tokens, `state.js`, the
  mock's rows, the `flavors` row field `interface_check`, the `interface/` fixture's second reader),
  `README.md` (the window's rows in the update feature and the interface paragraph),
  `crates/mrgl-core/tests/fixtures/README.md` (the window's test reads `interface/`).

## Deviations from the plan

- The marks' words became one constant each in the core, and the command line's line mark uses them, so
  the window and `scan`/`list` cannot drift apart; the command line's output is unchanged.
- The warning's line is the title of the mark itself, not of the whole row: the row's other cells keep
  their own titles (the source badge, the version note, the Forever note).
- The legend lists the two marks after the five states.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green, among them the 15 of `tests/flavor_list.rs` (the new
  interface test included), core unit 189, CLI `tests/cli.rs` 127, and the 74 Node tests of `tests/ui/`
  through `ui_smoke.rs`. `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check`: clean.
- 2026-10-22 — The sketch (`ui/index.html` served locally with `mock-data.js`) in the light and the dark
  theme: retail shows `out of date` under `managed` on My Unit Frames and `interface mismatch` under
  `unmanaged` on Old Minimap, each title the core's line, and the legend lists both marks.
- 2026-10-22 — The window (`target/debug/mrgl-app.exe`) with `MRGL_ROOT` on a temporary copy of the
  `interface/` fixture and `MRGL_HOME` on an empty temporary folder, read over the web view's debug port:
  `Behind` with `out of date` (title `Behind: interface 120100 is out of date for the build 12.1.5.70448
  (120105)`), `Current` without a mark, `Elsewhere` with `interface mismatch` (title `Elsewhere: interface
  110207 is not for the build 12.1.5.70448`), in both themes. No real game folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
