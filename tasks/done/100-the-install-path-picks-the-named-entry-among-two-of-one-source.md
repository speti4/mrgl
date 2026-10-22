---
size: S
origin: done/089-update-one-reports-a-second-entry-of-the-same-source.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 100 — The install path picks the named entry among two sidecar entries of one source

## Context

When two sidecar entries carry the same source, the install path and `update <name>` work on the
entry of the addon that was named, not on whichever entry's name sorts first; `managed_entry` looks up by
source and name together where a name is known.
Why: with a renamed addon next to its old name, `update <name>` can update the other entry today, since
`managed_entry` finds the first entry of the source alone.
Done when: a fixture with two entries from one source, the old name sorting first, updates the named
entry under `update <name>` and leaves the other alone, in the text and the JSON.

Decided 2026-10-22: the second of three follow-ups on one branch (099, 100, 102), in id order.

## Docs that move

- `docs/development.md`: the install engine's entry lookup, if it describes it

## Not in this change

- Merging or removing duplicate entries; the window's reporting of the second entry (099)

## Steps

1. [x] `install.rs`: `managed_entry` looks up by source and name where a name is known, by source alone
       otherwise; the callers of the one install path hand the name through
2. [x] Tests: a fixture with two entries from one source, the old name sorting first; `update <name>` updates
       the named entry and leaves the other alone, text and JSON; `update --all` unchanged

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `crates/mrgl-core/src/install.rs`: `managed_entry` takes the entry of the given name when its
  source is the spec, else the first entry of the source, else the entry of the name (ASCII
  case-insensitive throughout). `Engine::install_from_source` hands it the request's name (it passed none
  before); `Engine::target_for` passed the name already, so the channel of the pick follows the same entry
  now. The requests that carry a name are `update`'s (the sidecar's name, command line and window) and
  `import`'s (the list's name); `add <spec>` and `Engine::install_pick` carry none and look up by the source
  alone, as before.
- 2026-10-22 — tests: unit tests in `install.rs` for the lookup (the named entry, no name, a name of another
  source, a name alone) and for `AlreadyInstalled::update_line` of task 099 (both lines).
  `crates/mrgl-cli/tests/reports.rs`: the fixture of task 089 becomes `OldOneFolder` at 1.2.3 and
  `OneFolder` at 1.2.2, both `github:Questie/Questie`, the asset of 1.2.3 the fixture zip of `OneFolder`;
  `update OneFolder --yes` and `update --all --yes`, text and JSON. `crates/mrgl-app/tests/update_actions.rs`:
  the row update over the two entries of task 099 now updates `OneFolder`; the window's `installed-already`
  phase is tested through another run that updates the addon between the check and the install.
- 2026-10-22 — docs: `docs/development.md` (the install path's lookup, and when `update` still meets a
  second entry of the same source).

## Deviations from the plan

- The fixtures of task 089 (command line) and task 099 (window) answered "installed already" through the
  other entry; with the named entry found first they update the named addon, so both tests now assert this
  task's result. `update --all` changes with them on that fixture: it passes the sidecar's names as `update
  <name>` does, so the named entry is updated there too; every other `update --all` test is unchanged.
- The second-entry line of tasks 089 and 099 stays for the case the lookup still falls back to the source
  alone (no entry of the addon's name has the source any more, as after an install earlier in the run took
  its folders over); its words are covered by the core's unit test, the window's phase by the test of
  another run's update between the check and the install.

## Verification

- 2026-10-22 — `tests/reports.rs`: `mrgl --serious update OneFolder --yes` says `updating OneFolder from
  1.2.2 to 1.2.3`, `updated OneFolder from 1.2.2 to 1.2.3 (1 folder: OneFolder) in _forever_` and `1 addon
  updated`, no `not updated` line; `update --all --yes` ends with `1 addon updated, 1 up to date`. With
  `--json` both give `updated: 1`, `installed_already: 0`, and on `OneFolder` `updated: true` with the
  `addon` block of `OneFolder` 1.2.3. After every run the sidecar has `OneFolder` at 1.2.3 with its folder,
  and the `OldOneFolder` entry and folder as they were.
- 2026-10-22 — `tests/update_actions.rs`: the window's row update of `OneFolder` over the same shape goes
  `checking`, `checked`, `updating`, `downloading`, `installing`, `updated`, ends with `1 addon updated`,
  and leaves `OldOneFolder`'s entry and folder alone. The other run's update between the check and the
  install gives `installed-already` with `OneFolder 1.1.0 is installed already in _forever_ from
  github:Example/One, nothing to do` and `nothing updated: 1 installed already`.
- 2026-10-22 — with the install path's lookup put back to the source alone, the command line test and the
  window's named-entry test fail; with the change they pass.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green, the Node UI tests through `tests/ui_smoke.rs`), `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
