---
size: S
origin: wip/096-the-curseforge-source-with-the-users-key.md
created: 2026-10-22
started: 2026-10-24
closed: 2026-10-24
plan: none
milestone: M2 Window MVP
---

# 116 — The window installs a zip in hand with a recorded source

## Context

The window's add action takes a zip from disk and a source spec to record with it, the pair of the command
line's `add <zip> --as <spec>` (096), so a distribution-off CurseForge addon downloaded from its page is
installed and managed from the window; the row then carries the source like any other.
Why: 096 gave the command line the zip-in-hand way for a project that allows no download outside CurseForge,
and the window has no way to do the same.
Done when: a window test installs the 096 zip in hand with `curse:<id>` recorded, the row shows the source, and
the next update check asks that source.

Decided 2026-10-24: the third of three follow-ups on one branch (114, 115, 116).

## Docs that move

- `docs/development.md`: the window's add action and its commands, where it lists them
- `README.md`: the window's paragraph, if it lists what the window installs

## Not in this change

- A window field for the CurseForge key; fingerprint matching
- The command line (096 did `add <zip> --as <spec>`)

## Steps

1. [x] `crates/mrgl-app/src/actions.rs` (and the command it backs): the add action takes a zip path and an
       optional source spec, recorded with the install as the command line's `--as` does (any spec but a Wago
       page address; the version is the TOC's)
2. [x] The UI: the add form offers the source spec next to the zip pick; the row shows the recorded source
       with its badge like any other
3. [x] Tests: a window test installs `MrglNoDistribution-2.0.0.zip` with `curse:4244` recorded, the row shows
       the source, the next update check asks `curse:4244`; the Node tests of the UI modules

## Log

- 2026-10-24 — promoted
- 2026-10-24 — `crates/mrgl-app/src/actions.rs`: `add_zip` installs a zip from disk through the engine,
  recorded with the source `zip_source` reads from the form's field: a spec in its canonical spelling (any
  spec but a Wago page address, refused as `--as` refuses it), or the zip's path when the field is empty, as
  `mrgl add <file.zip>` records it; the version is the TOC's. The answer is the flavor as it is now, the
  warnings (the CurseForge app's marker first, the install's own, the missing dependencies and the interface
  check, as after an update) and the summary line. Without an AddOns folder it says to start the client
  once.
- 2026-10-24 — `crates/mrgl-core/src/install.rs`: `Installed::summary_line`, the install's summary line the
  command line built in `install_summary` (`installed … into …`, `updated … from … to … in …`,
  `reinstalled … in …`, the removed folders), moved into the core so the window says the same words; the
  command line calls it.
- 2026-10-24 — `crates/mrgl-app/src/commands.rs`: `choose_zip`, the native file dialog with a `.zip`
  filter (a reasoned `coverage(off)` exclusion like `choose_root`, listed in the docs), and `add_zip`, which
  holds the update run while it installs (refused while an update runs, `an update or an add is running;
  the zip waits until it ends`, and while a toolbox write goes, `toolbox::add_waits_line`); both registered
  in `lib.rs`.
- 2026-10-24 — the UI: "Add a zip…" in the flavor header opens the file dialog, then a form with the zip's
  name, an optional source field and "Install"; the add disables the update buttons while it goes, and its
  answer redraws the flavor, where the row carries the recorded source's badge as any other row
  (`list::badge_of` already reads every spec). `actions.js` has the pure part (`addZipButton`,
  `addZipRequest`, `fileName`), `data.js` the two commands, `mock-data.js` a made-up dialog and add.
  Checked in the offline sketch in a browser: the form over the list, and after "Install" the row
  `MrglNoDistribution` with the `curseforge` badge and the summary toast.
- 2026-10-24 — tests: `tests/update_actions.rs`
  `the_add_form_installs_a_zip_in_hand_with_its_source_and_the_next_check_asks_it`,
  `the_add_form_records_the_path_without_a_source_and_refuses_a_page_or_a_bad_spec` and
  `the_add_form_warns_as_an_install_path_does_and_says_a_failed_install`; `tests/commands.rs`
  `an_add_waits_for_an_update_and_for_a_toolbox_write` and
  `the_add_command_installs_on_the_engine_of_the_process` (the mock runtime, the environment in a child);
  the Node tests of `actions.js`, `data.js` and `mock-data.js`.
- 2026-10-24 — docs: `docs/development.md` (the header's "Add a zip…", the contract's two calls, the pure
  module's add form, the commands `choose_zip` and `add_zip`, the window's install paths, the coverage
  exclusion list, the command line's `--as` and the CurseForge paragraph), `README.md` (the window's
  paragraph).

## Deviations from the plan

- The install's summary line moved from the command line into the core (`Installed::summary_line`), so the
  window's add says the command line's words instead of a second copy of them.
- The add holds the update run's guard: an add and an update both install through the engine and write the
  sidecar, so they take turns; a second command for a separate guard was not needed.

## Verification

- 2026-10-24 — `tests/update_actions.rs`: the add of `MrglNoDistribution-2.0.0.zip` with ` curse:4244 `
  answers the one message `installed MrglNoDistribution 2.0.0 (1 folder: MrglNoDistribution) into
  _forever_` (a success); the row is managed at 2.0.0 with the badge `{ kind: curseforge, id: 4244 }`, the
  sidecar records `curse:4244` and 2.0.0, and the next check asks the stub source `curse:4244` alone and sums
  up as `checked _forever_: 1 only on CurseForge`. A Wago page address and an unknown spec are errors and
  install nothing; an empty field records the zip's path (badge `zip`, `one-folder-1.0.0.zip`), the same zip
  again is `reinstalled OneFolder 1.0.0 (1 folder: OneFolder) in _forever_`. With the `.curseclient` marker
  the marker's warning comes first, then the interface warning of `stale-interface-1.0.0.zip`, then the
  summary; a missing zip is the engine's error; a flavor without an AddOns folder says to start the client.
- 2026-10-24 — `tests/commands.rs`: `add_zip` is refused with its warnings while an update run or a toolbox
  write holds its guard; in a child with `MRGL_ROOT` and `MRGL_HOME` it installs the zip with `curse:4244`
  on the process's flavor, ends the run, and answers the error of a flavor the root does not have.
- 2026-10-24 — `node --test crates/mrgl-app/tests/ui/*.test.js`: 91 tests green; the offline sketch served
  with `python -I -m http.server` and driven in a browser showed the form and the added row.
- 2026-10-24 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked`, `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check` green;
  the coverage measurement of CONTRIBUTING.md (cargo-llvm-cov on the pinned nightly) over the branch with
  tasks 114 to 116, run from a short substituted drive path (from the worktree's long path the window crate
  is not instrumented), with the `tests/` files and the build scripts left out of the export as the CI run
  leaves them out: 100.00% lines (0 missed of 15692) and branches (0 missed of 2288); mrgl-app 1848 / 138,
  mrgl-cli 2496 / 390, mrgl-core 11348 / 1760; 5 reasoned exclusions in the code and in the docs list.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
