---
size: S
origin: done/082-install-and-update-report-the-interface-check.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 090 — `import` runs the dependency check after an install, as `add` and `update` do

## Context

`mrgl import` reports the missing dependencies of every addon it installed, through the same
`Engine::check_installed` the install summary of `add` and `update` reads (079, 082), in the text and the
JSON next to the interface warnings it already prints.
Why: an imported list is the one path that installs many addons at once, and the one that says nothing
when a required library is not on the list.
Done when: a fixture list that installs an addon with a missing dependency prints the warning and carries
it in the JSON, and a complete list prints none.

Decided 2026-10-22: the third of three wave-7 follow-ups on one branch (088, 089, 090), in id order.

## Docs that move

- `docs/development.md`: the `import` command's summary, the dependency warning
- `README.md`: the import usage, if it describes what the summary reports

## Not in this change

- Installing a missing dependency on its own (the warning names it, as `add` does)

## Steps

1. [x] `main.rs`: `import` runs `Engine::check_installed` over what it installed and prints the missing
       dependencies next to the interface warnings, in the text and the JSON
2. [x] Tests: a fixture list with a missing dependency prints the warning and carries it in the JSON; a
       complete list prints none

## Log

- 2026-10-22 — promoted
- 2026-10-22 — `crates/mrgl-core/src/install.rs`: `Engine::check_installed_each` gives the checks of
  `check_installed` for several installs over one scan and one dependency check of the flavor;
  `check_installed` is that call for one install.
- 2026-10-22 — `crates/mrgl-core/src/list_file.rs`: once every entry is done, `import` runs
  `check_installed_each` over the folders of the entries it installed, in place of its own scan for the
  interface check: the interface mismatch and out-of-date state as before, the new `Imported::dependencies`
  (the check of each installed folder), and each missing dependency's warning line (and a scan that failed)
  in the entry's `warnings`, which the command line already prints and puts in the document's `warnings`.
  A library installed later on the list counts as there for an addon earlier on it.
- 2026-10-22 — test in `crates/mrgl-cli/tests/reports.rs`: a list of `NeedsLibrary` (the fixture zip
  `needs-library-1.0.0.zip` behind the stub's `Questie/Questie` asset) into an empty flavor and into one
  that has `LibEmbedded` and `LibMissing` as addons of their own, text and JSON.
- 2026-10-22 — docs: `docs/development.md` (the import paragraph, the dependency check's list of commands
  and its JSON), `README.md` (the dependency check paragraph names `update` and `import`).

## Deviations from the plan

- The check runs in the core's `list_file::import`, where the interface check of the installed entries
  already ran, not in `main.rs`: the command line prints the entries' warnings as it did, so `main.rs`
  needed no change.
- One scan for all installed entries (`Engine::check_installed_each`) instead of one
  `Engine::check_installed` call per entry; `check_installed` is the same code for one install.

## Verification

- 2026-10-22 — `tests/reports.rs`: into an empty flavor, `mrgl --serious import list.json --flavor
  forever` prints `NeedsLibrary  installed 1.2.3 from github:Questie/Questie`, then `NeedsLibrary needs the
  missing dependencies LibEmbedded, LibMissing: install them or the addon does not load (LibEmbedded is
  only embedded inside NeedsLibrary, …)`, then `imported 1 addon of list.json into _forever_: 1
  installed`; `--json` carries the line in the entry's `warnings` and the document's, and the entry's
  `dependencies` names both as missing. With both libraries in the flavor no warning is printed, both
  `warnings` are empty and `dependencies` lists them as met. With the core's change set aside, the same
  test failed: the text had no dependency line.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green; the import, interface and starter-list tests of `tests/cli.rs` unchanged),
  `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
