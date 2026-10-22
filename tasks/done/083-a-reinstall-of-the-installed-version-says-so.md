---
size: S
origin: done/077-marks-survive-a-reinstall-and-the-pinned-line-orders-versions.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 083 — A reinstall of the installed version says it was reinstalled, not updated from the previous version

## Context

When a zip in hand or a pick installs the version the sidecar already records, the install summary says
`reinstalled <name> <version>` instead of `updated <name> from <previous> to <same version>`, in the text
and as an `outcome` in the JSON; the entry's previous version stays what it was (077).
Why: `Installed::previous_version` carries the entry's older version, so a reinstall reads as an update
between two different versions that are the same.
Done when: a fixture reinstall of the installed version prints the reinstalled line and keeps the previous
version for `rollback`, and a real update still prints `updated … from … to …`.

Decided 2026-10-22: runs after 082 on the same branch, since both change the command line's install summary.

## Docs that move

- `docs/development.md`: the install summary's outcomes (`installed`, `updated`, `reinstalled`) and the JSON key
- `README.md`: only if an example shows the reinstall line

## Not in this change

- The commit rule of 077 (which entry a staged name replaces) and the previous version it keeps
- The window's wording of an update answer, unless it prints the same core line

## Steps

1. [x] The core's install result tells a reinstall of the recorded version apart from an update between two
       versions (an outcome the summary and the JSON read), with the previous version kept as 077 keeps it
2. [x] `mrgl-cli/src/main.rs`: the summary line `reinstalled <name> <version>` through the voice, the JSON
       `outcome`; `updated … from … to …` unchanged for a real update
3. [x] Tests: a fixture reinstall of the installed version prints the reinstalled line and `rollback` still
       finds the previous version; a real update prints the updated line (`tests/install.rs`, `tests/cli.rs`)
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — core: `install::InstallOutcome` (`installed`, `updated`, `reinstalled`, kebab-case in JSON)
  with `InstallOutcome::of(replaced, version)`, told by the entry the commit replaces (none, another
  version, the same version), and `InstallOutcome::previous_version`, the rule `Engine::commit` already
  applied (the replaced entry's version on an update, its own previous version on a reinstall). `Installed`
  carries `outcome`; the commit's entry, the cache it prunes and the rollback are unchanged.
- 2026-10-22 — CLI: the install summary's line follows the outcome: `reinstalled <name> <version>
  (<folders>) in <flavor folder>` for a reinstall, `updated … from … to …` for an update, `installed …
  into …` for a first install; the `addon` block carries `outcome` (`null` for an addon found installed
  already). `add <file.zip> --dry-run` applies the same rule (`would reinstall OneFolder 1.1.0 …`) and its
  `addon` block carries the outcome and the previous version the install would record, where it named the
  recorded version as the previous one before.
- 2026-10-22 — tests: `tests/install.rs` `a_reinstall_of_the_recorded_version_is_told_apart_from_an_update`
  (the four outcomes over `one-folder` 1.0.0, 1.0.0, 1.1.0, 1.1.0, the entry's previous version and the
  rollback to 1.0.0) and an outcome assertion in `reinstalling_the_same_version_keeps_the_previous_one`;
  `tests/cli.rs` `a_reinstall_of_the_installed_version_says_so_and_keeps_the_rollback` (the reinstall of a
  first install, the update line, the dry run, the murloc line, the JSON `outcome` and `previous_version`,
  the sidecar and `rollback`); the `add github:` JSON test's `addon` block gains `outcome: installed`.
- 2026-10-22 — docs: `docs/development.md` (the summary line's three outcomes, the dry run's words and the
  `outcome` key of the `addon` block). `README.md` shows no reinstall line and is unchanged.

## Deviations from the plan

- The dry run of `add <file.zip>` follows the same rule, and its `addon` block now names the previous
  version the install would record instead of the recorded version: without it the dry run would still
  say `would update OneFolder from 1.1.0 to 1.1.0`.
- A reinstall of a first install (no previous version) says `reinstalled` too, not `installed`.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (`tests/cli.rs` 131, was 130; `tests/install.rs` 38,
  was 37; core unit 189; the window's tests unchanged). `cargo fmt --check`, `cargo clippy --all-targets
  --locked -- -D warnings`, `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py
  --check` green.
- 2026-10-22 — in a temporary game folder and `MRGL_HOME`, `mrgl --serious add` of `one-folder` 1.0.0,
  1.1.0 and 1.1.0 again: `installed OneFolder 1.0.0 (1 folder: OneFolder) into _forever_`, `updated OneFolder
  from 1.0.0 to 1.1.0 (1 folder: OneFolder) in _forever_`, `reinstalled OneFolder 1.1.0 (1 folder:
  OneFolder) in _forever_`; then `rollback OneFolder`: `rolled back OneFolder from 1.1.0 to 1.0.0 (1 folder:
  OneFolder) in _forever_; pinned at 1.0.0, …`. No real game folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
