---
size: S
origin: done/109-the-windows-adopt-view.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
changelog: housekeeping
---

# 112 — The toolbox's adopt lines in the window's words, without the command line's flags

## Context

The core's adopt summary, marker line and refusal take a caller's wording, as `CleanupBy` does for the
SavedVariables summary (086), so the toolbox's adopt card says "the Adopt button adopts N of them" and names
its confirmation instead of `adopt --apply` and `--alongside-curseforge`; the command line keeps its flags.
Why: 109 shows the core's lines as they are, and three of them name command-line flags the window has not.
Done when: the card's three lines name no flag, the command line's are unchanged, and the Rust and Node tests
assert both wordings.

Decided 2026-10-22: the first of two adopt-card follow-ups on one branch (112, 113), on the branch of 110
and 109.

## Docs that move

- `docs/development.md`: the adopt lines' caller wording, next to `CleanupBy`

## Not in this change

- Any new adopt line; the apply's requests (113)

## Steps

1. [x] `adopt.rs`: the summary line, the marker line and the refusal take a wording value (the command line's
       flags, the window's button and confirmation), as `CleanupBy` does
2. [x] The command line passes its wording (unchanged output); the toolbox passes the window's
3. [x] Tests: the CLI tests unchanged, the window tests and the Node tests assert the window's three lines

## Log

- 2026-10-22 — promoted
- 2026-10-22 — core: `adopt::AdoptBy` (`Command`, `Window`) names what adopts, as `savedvars::CleanupBy`
  names the way to clean up: `Report::summary(by)`, `Report::marker_line(by)` and
  `adopt::marker_refusal(flavor, by)` take it. `Command` keeps `adopt --apply` and `--alongside-curseforge`;
  `Window` reads `the Adopt button` and `the confirmation to adopt alongside the CurseForge app` (the
  dialog's button): `…; the Adopt button adopts 5 of them`, `… so the Adopt button writes nothing there
  without the confirmation to adopt alongside the CurseForge app`, `…: the Adopt button wrote nothing; the
  confirmation to adopt alongside the CurseForge app adopts its addons anyway, …`.
- 2026-10-22 — callers: `mrgl adopt` passes `AdoptBy::Command` (its output unchanged), the toolbox's
  `adopt_view` and `adopt_apply` pass `AdoptBy::Window`; the sketch's `mock-data.js` says the window's
  lines too.
- 2026-10-22 — tests: `crates/mrgl-core/tests/adopt.rs` asserts both wordings of the three lines;
  `crates/mrgl-cli/tests/adopt.rs` unchanged; `crates/mrgl-app/tests/adopt.rs` asserts the window's summary,
  marker line and refusal and that the card's lines hold no `--`; the Node tests
  (`mock-data.test.js`, `toolbox.test.js`) assert the window's lines and no `--` in them.
- 2026-10-22 — docs: `docs/development.md`, the shared summary lines of the voice section (`AdoptBy` next
  to `CleanupBy`) and the Adopt section (the window's words).

## Deviations from the plan

- none

## Verification

- 2026-10-22 — `cargo test --locked`: every suite green, among them `crates/mrgl-core/tests/adopt.rs`,
  `crates/mrgl-cli/tests/adopt.rs` (the command line's lines unchanged), `crates/mrgl-app/tests/adopt.rs` (6)
  and `ui_smoke.rs`, whose `node --test` runs the 87 Node tests of `tests/ui/`. `cargo fmt --check`,
  `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check --all-targets --locked` and
  `python -I scripts/changelog.py --check` green; `tasks_check` without `write`: only the stale index,
  which the main session regenerates.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
