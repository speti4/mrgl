---
size: S
origin: wip/096-the-curseforge-source-with-the-users-key.md
created: 2026-10-22
started: 2026-10-24
closed: 2026-10-24
plan: none
milestone: M4 Sources
---

# 115 — A distribution-off CurseForge entry is a state of its own in the update check

## Context

A `curse:` entry whose project allows no download outside CurseForge (`allowModDistribution: false`) gets its
own state in the update check and the window's row, as the pending state of 104, counted apart from the
failed checks: the line names the page and the zip-in-hand way (`add <zip> --as curse:<id>`), and a run with
such entries only ends with exit code 0.
Why: 096 reports it as a failed check, which turns every run red for a choice the author made, not an error.
Done when: the distribution-off fixture (id 4244) ends an `update --all` with exit code 0 and its own line and
JSON state, the window's row shows it, and a failed check still ends with 1.

Decided 2026-10-24: the second of three follow-ups on one branch (114, 115, 116).

## Docs that move

- `docs/development.md`: the update check's states (text and JSON) and the window's row states
- `README.md`: the CurseForge paragraph's distribution-off line, if it says the check fails

## Not in this change

- The 096 task file (another session's open task; its recorded deviation on this state stays as written)
- The window's own zip-in-hand install with a source (116)

## Steps

1. [x] `crates/mrgl-core/src/update.rs` (and the source's error): `Error::DistributionOff` becomes a state of
       its own in the check's outcome, counted apart from the failed checks, with the line that names the
       page and `mrgl add <file.zip> --as curse:<id>`
2. [x] `crates/mrgl-cli/src/main.rs`: the text line through the voice, the JSON state and count, exit code 0
       for a run whose only non-success is this state; a failed check still ends with 1
3. [x] `crates/mrgl-app`: the window's row shows the state (as the pending state of 104), the summary counts
       it; the offline check's last answers keep the state
4. [x] Tests: the 4244 fixture under `update --all` (text, JSON, exit code), a failed check next to it with
       exit 1, the window's row, the Node tests of the UI modules

## Log

- 2026-10-24 — promoted
- 2026-10-24 — `crates/mrgl-core/src/update.rs`: `UpdateState::DistributionOff` (`distribution-off` in the
  JSON, the label `only on CurseForge`), listed in `UpdateState::ALL` after `pending`. The check maps
  `Error::DistributionOff` of an entry that is not pinned onto it, with the source's line as the reason, so
  the data line reads `MrglNoDistribution  2.0.0  only on CurseForge: the author of curse:4244 (Mrgl No
  Distribution) allows no download outside CurseForge: download the zip from <page> and install it with
  `mrgl add <file.zip> --as curse:4244``; a pinned entry stays `pinned`, as for any error. The summary
  counts it apart (`1 only on CurseForge`) and does not make it a warning.
- 2026-10-24 — `crates/mrgl-core/src/last_check.rs`: `is_kept` keeps the state, since the CurseForge source
  keeps no cache and the window's offline view has no key: only the kept answer tells it after a restart.
- 2026-10-24 — `crates/mrgl-cli/src/main.rs` needed no change: `update` prints the check's data line as it
  prints every state's, the JSON's `counts` follow `UpdateState::ALL` (a `distribution-off` count), and only
  `failed` checks and failed installs end a run with exit code 1.
- 2026-10-24 — the window: the row's `check` carries the state and its label from the core; the note under
  a row's version names it (`actions.js`'s `NOTED` gained `distribution-off`), and the offline sketch's
  labels, data line and summary order (`mock-data.js`) know it.
- 2026-10-24 — tests: core unit `a_project_without_distribution_is_a_state_of_its_own_unless_pinned` and the
  line in `every_state_has_its_line`, the kept state in `a_curse_entrys_answer_keeps_no_version`, the state of the check in
  `tests/curseforge_source.rs` `a_project_without_distribution_names_its_page_and_installs_from_a_zip_in_hand`; the
  command line's `add_curse_names_the_page_of_a_project_without_distribution_and_takes_its_zip_with_as`
  (`update --all --dry-run` and `--yes` over the 4244 zip in hand: exit code 0, the line, the summary, the
  JSON state, detail and count, the kept answer in `last-check.json`; a `curse:4245` entry next to it, which
  answers 401, ends the run with exit code 1 and both lines) and the `distribution-off` count in
  `update_json_is_one_document_with_every_state_and_count`; the window's `tests/update_actions.rs`
  `a_project_without_distribution_is_a_state_of_its_own_on_its_row_and_after_a_restart`; the Node test of
  `noteOf`.
- 2026-10-24 — docs: `docs/development.md` (the update states, the kept answers of the last check, the
  states the window's row note names, the CurseForge paragraph under Fixtures), `README.md` (the states of
  the update check, the CurseForge paragraph's distribution-off line).

## Deviations from the plan

- The command line's code is unchanged (step 2): its data line, its JSON counts and its exit code rule
  already follow the core's states, so the new state reached them without a change; the step's tests were
  written as planned.
- The 401 project `curse:4245` is the failed check next to the distribution-off entry, recorded on the
  fixture zip `one-folder-1.0.0.zip` with `--as curse:4245`, since no other CurseForge fixture fails to answer.

## Verification

- 2026-10-24 — `tests/cli.rs`: `mrgl --serious update --all --dry-run` over the zip in hand recorded as
  `curse:4244` ends with exit code 0, prints `MrglNoDistribution  2.0.0  only on CurseForge: the author of
  curse:4244 (Mrgl No Distribution) allows no download outside CurseForge: download the zip from
  https://www.curseforge.com/wow/addons/mrgl-no-distribution and install it with `mrgl add <file.zip> --as
  curse:4244`` and ends with `dry run: 1 only on CurseForge; nothing written`; `update --all --yes` ends with
  0 and `nothing updated: 1 only on CurseForge`; `--json` has `state: distribution-off`, the line as
  `detail`, `latest: null`, `counts.distribution-off: 1` and `counts.failed: 0`, and `last-check.json` keeps
  the state. With a `curse:4245` entry next to it the run ends with exit code 1, `OneFolder  1.0.0  check
  failed: CurseForge answered 401 …` and `dry run: 1 only on CurseForge, 1 check failed; nothing written`.
- 2026-10-24 — `tests/update_actions.rs`: the window's "update all" over the same entry sends `checking` and
  `checked` with the state `distribution-off`, the label `only on CurseForge` and the line, sums up as
  `nothing updated: 1 only on CurseForge` (kind `nothing`), and the row keeps the state, also in the offline
  list a restart draws.
- 2026-10-24 — `node --test crates/mrgl-app/tests/ui/*.test.js`: 88 tests green. `cargo fmt --check`,
  `cargo clippy --all-targets --locked -- -D warnings` and `cargo test --locked` green.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
