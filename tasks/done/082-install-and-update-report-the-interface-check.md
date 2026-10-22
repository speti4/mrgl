---
size: S
origin: done/079-window-and-exe-polish-after-the-first-release.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 082 — An install and an update report the whole interface check: out of date next to the mismatch, and `update` says it at all

## Context

`add <file.zip>`, `import` and `update` warn after an install when the addon's TOC is behind the build
(`out of date`, 076) as they warn about a mismatch, in the text and in the JSON (`interface_out_of_date`
next to `interface_mismatch`); and `mrgl update` prints the interface warning it computes in
`install_summary` and carries it in its JSON document, which it drops today.
Why: after a client update every updated addon with the previous interface number shows `Out of date` in the
client while the install summary says nothing, and `update` says nothing about a mismatch either.
Done when: the CLI tests of `add`, `import` and `update` over the `stale-interface` and a behind-the-build
fixture zip see both warnings in the text and the JSON.

Decided 2026-10-22: runs as the first task of a chain with 083 and 087 on one branch, since all three change
the command line's install summary and `update` output.

## Docs that move

- `docs/development.md`: the install summary's warnings (both interface states) and the `update` JSON document
- `README.md`: the `out of date` mark where the install warnings are described, if the text names them

## Not in this change

- The window's rows (081) and the window's update answer, which already carries the core's checks (079)
- A reinstall's own wording (083) and the last-check file (087)

## Steps

1. [x] The core's installed checks (`Engine::check_installed`, `InstalledChecks::lines`) carry the out-of-date
       state next to the mismatch, with one warning line each through the voice
2. [x] `mrgl-cli/src/main.rs`: `add <file.zip>`, `import` and `update` print both warnings and carry
       `interface_out_of_date` next to `interface_mismatch` in their JSON
3. [x] Tests in `crates/mrgl-cli/tests/cli.rs` over the `stale-interface` zip and a behind-the-build fixture
       zip: both warnings in the text and the JSON, for `add`, `import` and `update`
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — core: `InstalledChecks` carries `out_of_date` (`InterfaceOutOfDate`) next to `mismatch`;
  `Engine::check_installed` runs `interface::out_of_date_addon` on the primary folder beside
  `interface::check_addon`, and `InstalledChecks::interface_lines` gives the one interface line, which
  `lines` appends after the dependency warnings. `InterfaceOutOfDate::after_install_line` is the new line:
  `BehindInterface: interface 16000 is out of date for the build 1.60.1.70245 (16001) of _forever_; the
  client lists it as out of date`. `list_file::Imported` carries `interface_out_of_date` next to
  `interface_mismatch`, computed on the same scan after the import.
- 2026-10-22 — CLI: the install summary (`InstallSummary`) keeps `out_of_date` next to `mismatch` and says
  either line through the voice (`say_interface`). `add <file.zip>` and `add github:` say it where they
  said the mismatch and carry `interface_out_of_date` in their JSON; `import` says it after each entry's
  mismatch, and its entries carry the key; `update` says the interface line under each updated addon's
  summary line, and every addon of its JSON document carries `interface_mismatch` and
  `interface_out_of_date` (`null` for an addon that was not updated). `Applied::Updated` holds a boxed
  summary (clippy's `large_enum_variant`).
- 2026-10-22 — fixture: `zips/behind-interface-1.0.0.zip` (`BehindInterface`, interface 16000, one patch
  behind the Forever fixture build 1.60.1), written by `make-zips.py`, which rewrites no other zip; the
  fixtures README names it.
- 2026-10-22 — tests: `tests/cli.rs` `add_reports_an_out_of_date_install_next_to_the_mismatch`,
  `import_reports_an_out_of_date_install_next_to_the_mismatch` and
  `update_reports_the_interface_check_of_an_updated_addon` (text and JSON over the `behind-interface` and
  `stale-interface` zips, the stub serving each behind the Forever asset of `Questie/Questie`);
  `tests/install.rs` `the_installed_checks_tell_out_of_date_from_a_mismatch`.
- 2026-10-22 — docs: `docs/development.md` (the after-install interface warning of `add`, `import` and
  `update` for both states, the JSON keys of the three documents, the `update` document's keys and its
  summary line), `README.md` (an install or an update says `out of date` for the addon it installed).

## Deviations from the plan

- `add github:` reports out of date too: it shares the install summary with `add <file.zip>` and `update`.
- The window's update answer gains the out-of-date line without a change of its own, since it reads
  `InstalledChecks::lines` (079); its rows are unchanged.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (`tests/cli.rs` 130, was 127; `tests/install.rs`
  37, was 36; core unit 189; the window's `flavor_list` 14 and `update_actions` 10 unchanged).
  `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.
- 2026-10-22 — `mrgl add behind-interface-1.0.0.zip --flavor forever` in a temporary game folder and
  `MRGL_HOME`: `installed BehindInterface 1.0.0 (1 folder: BehindInterface) into _forever_`, then the
  warning under its murloc line, `BehindInterface: interface 16000 is out of date for the build
  1.60.1.70245 (16001) of _forever_; the client lists it as out of date`. No real game folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
