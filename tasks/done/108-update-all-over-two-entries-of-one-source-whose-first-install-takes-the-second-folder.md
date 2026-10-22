---
size: S
origin: done/100-the-install-path-picks-the-named-entry-among-two-of-one-source.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 108 — `update --all` over two entries of one source whose first install takes the second entry's folder

## Context

When a release zip carries the folder of a second sidecar entry of the same source, the first
install takes that folder over and drops the second entry; `update --all` then reports the dropped entry
as folded into the first (its name, the folder, the entry it joined) instead of a "stays at <version>" line
about an entry that no longer exists, in the text and the JSON.
Why: after 100 the line for the second entry describes a sidecar entry the first install already removed.
Done when: a fixture with two entries of one source and a zip carrying both folders reports the second
entry as folded into the first under `update --all`, text and JSON, and the sidecar holds one entry after.

Decided 2026-10-22: the first of two follow-ups on one branch (108, 107).

## Docs that move

- `docs/development.md`: the `update --all` outcomes, if it lists them

## Not in this change

- The window's update over the same case (it reads the core's outcome once the core has one)

## Steps

1. [x] The install path reports the sidecar entries it folded into the installed one; `update --all` turns
       that into the "folded into" outcome for the dropped entry (through the voice) and the JSON
2. [x] Tests: a fixture with two entries of one source and a zip with both folders, `update --all` text and
       JSON, the sidecar with one entry after

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `crates/mrgl-core/src/install.rs`: `Installed::folded` names the entries a commit took every
  folder of and dropped, as the core's `Folded` (the dropped entry's name, version and folders, the addon it
  joined and that addon's version); `take_over_folders` returns them, its two warnings stay as they were.
  `Folded::update_line` is the line an update of the dropped entry says: `<entry> folded into <addon>:
  <addon> <version> ships its folder <folder> in <flavor folder>, <entry> has no entry of its own now`
  (`its folders A, B` for several), a success.
- 2026-10-22 — `crates/mrgl-core/src/update.rs`: `SummaryOf::Update` carries a `folded` count, said as
  `1 folded into another entry` and taken out of the failed count; the window's update passes 0
  (`crates/mrgl-app/src/actions.rs`, its own case stays out of this change).
- 2026-10-22 — `crates/mrgl-cli/src/main.rs`: `update` keeps the entries the installs of the run folded; an
  update of one of them installs nothing and is `Applied::Folded`: the line through the voice, the
  addon's JSON `folded_into` (`{ name, version, folders, message }`, `null` otherwise) and the document's
  `folded` count; the summary counts it, the exit code stays 0.
- 2026-10-22 — tests: `crates/mrgl-core/tests/install.rs` (an entry whose one folder an install takes over
  is folded into it, the line for one and for two folders; a partial takeover folds nothing), the summary
  unit test in `update.rs`, and `crates/mrgl-cli/tests/reports.rs`: `SeveralFolders` and
  `SeveralFolders_Options`, both `github:Questie/Questie` at 1.2.2, the asset of 1.2.3 the existing
  fixture zip `several-folders-2.1.0.zip`, which carries both folders; `update --all --yes`, text and JSON.
- 2026-10-22 — docs: `docs/development.md` (the `update --all` outcome and the JSON fields).

## Deviations from the plan

- The fixture zip is the existing `several-folders-2.1.0.zip` (two addon folders, written by
  `zips/make-zips.py`), so no new zip was made.

## Verification

- 2026-10-22 — `tests/reports.rs`: `mrgl --serious update --all --yes` says `updating SeveralFolders from
  1.2.2 to 1.2.3`, then `SeveralFolders_Options folded into SeveralFolders: SeveralFolders 1.2.3 ships its
  folder SeveralFolders_Options in _forever_, SeveralFolders_Options has no entry of its own now`, no
  `updating SeveralFolders_Options`, no `not updated` and no `stays at` line, and ends with `1 addon updated,
  1 folded into another entry`. With `--json`: `updated: 1`, `folded: 1`, `installed_already: 0`;
  `SeveralFolders` has `updated: true` and `folded_into: null`, `SeveralFolders_Options` has `updated:
  false`, `addon`, `installed_already` and `error` `null`, and `folded_into` with `SeveralFolders`, 1.2.3,
  its folder and the line. After both runs the sidecar holds the one entry `SeveralFolders` at 1.2.3 with
  both folders.
- 2026-10-22 — with the lookup of the folded entries turned off, the command line test fails on the old
  line `SeveralFolders_Options not updated: SeveralFolders, a second entry of github:Questie/Questie, has
  1.2.3 installed already in _forever_; SeveralFolders_Options stays at 1.2.2`; with it the test passes.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green, the Node UI tests through `tests/ui_smoke.rs`), `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
