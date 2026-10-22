---
size: S
origin: done/092-one-source-per-spec-the-router-and-the-generic-paths.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
changelog: housekeeping
---

# 102 — The window's offline sketch says "source not configured" as the core does

## Context

The mock data of the window's offline sketch (`crates/mrgl-app/ui/mock-data.js`) writes the core's
line for a row whose source this build lacks, `source not configured: <spec> (this build has no <Name>
source)`, and its Node tests assert the same words.
Why: the sketch still says "a source the update check does not handle yet", the line the core wrote
before the router of 092.
Done when: the sketch's row text matches the core's line word for word and a Node test asserts it.

Decided 2026-10-22: the third of three follow-ups on one branch (099, 100, 102), in id order.

## Docs that move

- none

## Not in this change

- The window's rows themselves (they show the core's line already); any new sketch row

## Steps

1. [x] `ui/mock-data.js`: the not-configured row's text is the core's line; the Node tests under
       `crates/mrgl-app/tests/ui/` assert it word for word

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `crates/mrgl-app/ui/mock-data.js`: a managed row of a source other than GitHub (the sketch's
  build has the GitHub source alone, as the core's router today) is not checked with the detail
  `source not configured: <spec> (this build has no <Name> source)`, the spec in the core's canonical
  spelling (`wago:`, `wowi:`, `curse:`) and the source's name as `SourceKind::name` writes it (`Wago`,
  `WoWInterface`, `CurseForge`); the row's data line reads `not checked: <that line>`, as
  `AddonUpdate::line` writes it.
- 2026-10-22 — test in `crates/mrgl-app/tests/ui/mock-data.test.js`: the check of the classic sketch,
  word for word for the WoWInterface and the Wago row, and no line of the old wording.

## Deviations from the plan

- none

## Verification

- 2026-10-22 — `node --test crates/mrgl-app/tests/ui/*.test.js`: 81 tests pass, among them the new one:
  `AtlasLoot Classic  v4.2.0  not checked: source not configured: wowi:24976 (this build has no
  WoWInterface source)` and `source not configured: wago:4N2mZ3KL (this build has no Wago source)`, the
  words of the router's own test in `crates/mrgl-core/src/sources/router.rs`.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green, the Node UI tests through `tests/ui_smoke.rs`), `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
