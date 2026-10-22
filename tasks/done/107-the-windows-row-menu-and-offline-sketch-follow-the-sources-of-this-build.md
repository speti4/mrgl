---
size: S
origin: done/099-the-windows-update-reports-a-second-entry-of-the-same-source.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
changelog: housekeeping
---

# 107 — The window's row menu and the offline sketch follow the sources of this build

## Context

The row menu's "why no update" line (`ui/actions.js`) says the core's words for a source this build
lacks or whose key is not set (`source not configured: …`, `needs <KEY NAME>`) instead of "the update check
does not ask this source yet", and the offline sketch (`ui/mock-data.js`) stops assuming a GitHub-only
build: its WoWInterface and Wago rows carry the answers the sources of 094 and 095 give; the Node tests
assert both.
Why: 092 to 095 made every source askable, and the sketch and the menu still describe the GitHub-only build.
Done when: the menu line and the sketch rows match the core's words for a WoWInterface, a Wago and a
not-configured row, asserted by the Node tests.

Decided 2026-10-22: the second of two follow-ups on one branch (108, 107).

## Docs that move

- `docs/development.md`: the offline sketch's description, if it says the sketch is GitHub-only

## Not in this change

- The window's rows themselves (they show the core's words); the pending WoWInterface state (104)

## Steps

1. [x] `ui/actions.js`: the "why no update" line takes the core's words from the row's check answer
2. [x] `ui/mock-data.js`: WoWInterface and Wago rows with the answers of 094 and 095, a not-configured row
       with the core's line; the Node tests under `crates/mrgl-app/tests/ui/` assert the three

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `crates/mrgl-app/ui/actions.js`: `whyNoUpdate` no longer refuses every source but GitHub. A
  linked row and a zip in hand keep their words; a row whose check answer is `not-checked` with a detail
  says that detail, the core's line (`source not configured: <spec> (this build has no <Name> source)`, or
  for Wago without its key `source not configured: wago:<id> (MRGL_WAGO_KEY is not set: …)`); a managed
  row without a source says the core's `not a source spec mrgl reads`; a WoWInterface or Wago row the check
  asked can be updated.
- 2026-10-22 — `crates/mrgl-app/ui/mock-data.js`: the sketch's build has the sources of the core's router,
  GitHub, WoWInterface and Wago (its key set), and no CurseForge source. The classic flavor's WoWInterface
  row (`AtlasLootClassic`, `wowi:24976`) is asked and has an update to `v4.2.1`, its Wago row (`TomTom`,
  `wago:4N2mZ3KL`) is asked and up to date, and its `Leatrix_Plus` row is managed from `curse:289009` and
  not checked with `source not configured: curse:289009 (this build has no CurseForge source)`.
- 2026-10-22 — tests: `crates/mrgl-app/tests/ui/actions.test.js` (the missing-key line and the
  no-CurseForge line as the menu's reason and the disabled update's title, a WoWInterface and a Wago row
  that can be updated); `crates/mrgl-app/tests/ui/mock-data.test.js` (the test of task 102 now asserts the
  three rows of the classic check, their data lines and their menu reason; the Forever and classic
  summaries count the Wago rows as asked).
- 2026-10-22 — docs: `docs/development.md` (the sketch's made-up check and the menu's reason).

## Deviations from the plan

- The sketch's not-configured row is a managed CurseForge row (`Leatrix_Plus` in classic, a guessed row
  before), since the sketch's build has every other source; the missing-key line of Wago is asserted by the
  menu's own test, not by a sketch row, as the sketch's run has the Wago key set.

## Verification

- 2026-10-22 — `node --test crates/mrgl-app/tests/ui/*.test.js`: 82 tests pass. The classic sketch's check
  asks `AtlasLootClassic` (`AtlasLoot Classic  v4.2.0 -> v4.2.1  update available`) and `TomTom` (`TomTom
  v5.5.4-mists  up to date`), and the menu gives neither a reason; `Leatrix_Plus` is not asked, its line is
  `Leatrix Plus  5.5.26  not checked: source not configured: curse:289009 (this build has no CurseForge
  source)` and its menu reason the core's line word for word. The menu's test gives the Wago missing-key
  line and the no-CurseForge line as the reason and as the disabled update's title, and no line of the old
  wording. The Forever run ends with `<n> addons updated, 1 up to date, 1 not checked`, the classic check
  with `checked _classic_: 2 updates available, …`.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green, the Node UI tests through `tests/ui_smoke.rs`), `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
