---
size: S
origin: done/082-install-and-update-report-the-interface-check.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 089 — `update <name>` says what happened when another entry of the same source already has the pick's version

## Context

When the one install path finds a second sidecar entry from the same source at the pick's version
and answers "installed already", `update <name>` reports that entry and the outcome instead of counting
the addon as updated and printing no line; the JSON carries the same.
Why: two sidecar entries from one source (a renamed addon next to its old name) make a silent, miscounted
update today.
Done when: a fixture with two entries from one source prints the line and counts the addon right, in the
text and the JSON, and `update --all` agrees.

Decided 2026-10-22: the second of three wave-7 follow-ups on one branch (088, 089, 090), in id order.

## Docs that move

- `docs/development.md`: the `update` command's outcomes, if it lists them

## Not in this change

- Merging or removing the duplicate entries (the line names them; a cleanup is a task of its own if wanted)

## Steps

1. [x] The install path's "installed already" answer carries the entry it found; `update <name>` and
       `update --all` report it as its own outcome, in the text (through the voice) and the JSON, with the
       counts right
2. [x] Tests: a fixture with two entries from one source, `update <name>` and `update --all`, text and JSON

## Log

- 2026-10-22 — promoted
- 2026-10-22 — the install path's answer already carried the entry it found (`AlreadyInstalled::name` and
  `entry`); `update` dropped it and counted the addon as updated. No change in `install.rs`.
- 2026-10-22 — `crates/mrgl-core/src/update.rs`: `SummaryOf::Update` gains `installed_already`, counted
  apart from the updated and the failed addons as `<n> installed already`, neither a success nor a warning
  on its own. Two more summary cases in the unit test.
- 2026-10-22 — `crates/mrgl-cli/src/main.rs`: `update`'s `Applied::InstalledAlready`. Under the addon's
  own entry it says the nothing-to-do line of `add` (kind: nothing to do); under another entry of the same
  source it says, as a warning, `<name> not updated: <entry>, a second entry of <source>, has <version>
  installed already in <flavor folder>; <name> stays at <version>`. The sidecar's warnings of that answer
  join the run's warnings. `--json`: every addon gains `installed_already` (the entry found as an `addon`
  block, with `same_entry` and the line as `message`, else `null`), its `updated` is `false` and its `addon`
  `null`; the document gains the `installed_already` count next to `updated`.
- 2026-10-22 — `crates/mrgl-app/src/actions.rs`: the window's update passes `installed_already: 0`, so its
  answer reads as before.
- 2026-10-22 — test in `crates/mrgl-cli/tests/reports.rs`: a sidecar with `Questie` at 1.2.3 and
  `QuestieClassic` at 1.2.2, both `github:Questie/Questie`, through `update QuestieClassic --yes` and
  `update --all --yes`, text and JSON.
- 2026-10-22 — docs: `docs/development.md` (the update run's outcomes and its JSON fields).

## Deviations from the plan

- Step 1's install path needed no change: `AlreadyInstalled` already named the entry it found. The fix sits
  in `update` and in the core's summary.
- The window's update (`crates/mrgl-app/src/actions.rs`) still counts such an answer as updated; it only
  passes the new field as 0. Listed as a follow-up in the pull request.

## Verification

- 2026-10-22 — `tests/reports.rs`: `mrgl --serious update QuestieClassic --yes` prints the check's lines,
  `updating QuestieClassic from 1.2.2 to 1.2.3`, then `QuestieClassic not updated: Questie, a second entry
  of github:Questie/Questie, has 1.2.3 installed already in _forever_; QuestieClassic stays at 1.2.2` and
  `nothing updated: 1 installed already`; `update --all --yes` the same line and `nothing updated: 1
  installed already, 1 up to date`. Before the change the same fixture through `update QuestieClassic`
  ended with `1 addon updated` and no line about the other entry. With `--json` both forms give `updated:
  0`, `installed_already: 1`, and on `QuestieClassic` `updated: false`, `addon: null` and the
  `installed_already` block naming `Questie` 1.2.3 with `same_entry: false`. The sidecar is unchanged after
  every run.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green), `cargo +1.90 check --all-targets --locked` and `python -I
  scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
