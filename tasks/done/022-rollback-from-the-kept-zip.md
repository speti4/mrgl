---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 022 — Rollback to the previous version from the kept zip; CLI `rollback`

## Context

`mrgl rollback <name>` installs the previous version's zip the install engine (004) kept in the cache
(`%LOCALAPPDATA%\mrgl\<flavor-id>\cache\<addon>\<version>.zip`, one previous version per addon) with the
same atomic swap (the engine's install call with the cached zip and the previous version as the
metadata), and pins the addon at that version (021). The swap's own revert on a failed install in 004 is
not this command. With daily broken builds after launch, "put yesterday's back" is one command instead of
a hunt for a zip.
Done when: an install, an update and a rollback in a temporary AddOns folder leave the first version's
files byte for byte, the sidecar shows the pin, and a rollback with no kept zip is a clear error.

Decided 2026-10-21: a rollback pins, so the next `update --all` does not reinstall the broken build;
`unpin` is the deliberate step back. The rollback reads the entry's `previous_version` and the cached zip
for it; the entry's source is kept; after the rollback the cache holds the rolled-back version (now
current) and the version rolled back from (now previous), so a second rollback goes forward again.

## Docs that move

- `README.md`: the `rollback` usage line next to `pin` and `unpin`.
- `docs/development.md` ("Layout"): the rollback path through the engine and the cache.

## Not in this change

- `update --all` (006), which reads the pin; a rollback across more than one version; the window's row
  action (010).

## Steps

1. [x] The core rollback: the entry's previous version and its cached zip, the engine's install call with
       the cached zip and that version as the metadata, the pin on the entry, the sidecar written
2. [x] Errors: not a managed addon, no previous version, the cached zip missing (a clear line each)
3. [x] `mrgl-cli`: `rollback <name> --flavor <id>`, the voice lines, `--json`
4. [x] Tests: install 1.0.0, update to 1.1.0, rollback; the files are 1.0.0's byte for byte, the sidecar
       pinned at 1.0.0, the cache holds both; a rollback with no kept zip is the documented error
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), on top
  of the branch of task 021 (the pin)
- 2026-10-21 — `install.rs`: `Engine::rollback(name)` reads the sidecar, reconciles it with the AddOns
  folder as an install does, finds the entry by name or by one of its folders, and takes its
  `previous_version` and `cache\<addon>\<previous>.zip`; the zip goes through the engine's own `stage` and
  `commit` with the metadata `source` (the entry's, unchanged), `version` (the previous one) and `name`
  (the sidecar's). The pin is set on the entry before the commit, which keeps the replaced entry's marks, so
  the sidecar is written once and no "pinned at another version" warning appears. It returns a
  `RolledBack` (name, version now, `previous_version` = the version rolled back from, `pinned`, `source`,
  folders, removed folders, warnings).
- 2026-10-21 — The cache: the engine's pruning already gives the decided result. The rolled-back zip is
  the cache file itself, so the copy is skipped (same file), and the prune keeps the version installed now
  and the entry's new previous version, the one rolled back from; nothing in the engine changed.
- 2026-10-21 — `error.rs`: `NoPreviousVersion` and `RollbackZipMissing` appended; an unmanaged folder reuses
  021's `NotManagedToMark` with the action `rolled back`, an unknown name is `AddonNotFound`. Every error is
  raised before anything is written.
- 2026-10-21 — `mrgl-cli`: `rollback <name> --flavor <id>`, a progress line (`rolling back <name> in
  _forever_ from the kept zip`), the success line naming both versions, the folders and the pin with the
  `unpin` hint, and `--json` (one document: `kind` `rollback`, `flavor`, `addon` with `name`, `version`,
  `previous_version`, `pinned`, `source`, `folders`, `removed_folders`, then `warnings`).
- 2026-10-21 — README: the `rollback` usage line after `unpin`; docs/development.md, "Layout": the rollback
  path through the engine and the cache, the command in the CLI list, and the cache line names `mrgl
  rollback`.

## Deviations from the plan

- The not-managed error reuses `Error::NotManagedToMark` of task 021 (action `rolled back`) instead of a
  new variant: the cause and the remedy are the same as for a pin.
- The name may be one of the addon's folders, as for `pin` and the other marks of 021 (a `list` line
  shows folders).
- A pin at another version is replaced by the rollback's pin; the ignored mark and the channel stay, as on
  any reinstall.
- An entry whose `previous_version` equals its installed version counts as having no previous version.

## Verification

- 2026-10-21 — `cargo test --locked`: every test green. `crates/mrgl-core/tests/install.rs`: one-folder
  1.0.0 installed, updated to 1.1.0, rolled back: every file under `OneFolder` has the SHA-256 of a fresh
  unpack of `one-folder-1.0.0.zip` (`old.lua` back, `new.lua` gone), the sidecar entry is version 1.0.0,
  `previous_version` 1.1.0, pinned at 1.0.0, the source unchanged and its file hashes 1.0.0's; the cache
  holds `1.0.0.zip` and `1.1.0.zip`, both byte-equal to the fixtures; the staging folder is empty; a second
  rollback goes forward to 1.1.0 (files equal to a fresh unpack of 1.1.0, pinned at 1.1.0, previous 1.0.0,
  the same two zips). several-folders 2.0.0 to 2.1.0 (drops `SeveralFolders_Data`), pinned at 2.1.0,
  ignored, prerelease, then rolled back by a folder name: the three folders are 2.0.0's byte for byte, the
  pin moves to 2.0.0, ignored and the channel stay, no warning; forward again removes
  `SeveralFolders_Data`. Errors: an unknown name, an unmanaged folder, no previous version and a deleted
  `1.0.0.zip` each give their documented line, the sidecar is byte-equal before and after, the AddOns
  files unchanged, the staging folder empty.
- 2026-10-21 — end to end through the exe (`crates/mrgl-cli/tests/cli.rs`): after `add` 1.0.0 and 1.1.0,
  `rollback onefolder` prints `rolling back onefolder in _forever_ from the kept zip` and `rolled back
  OneFolder from 1.1.0 to 1.0.0 (1 folder: OneFolder) in _forever_; pinned at 1.0.0` followed by the
  `mrgl unpin OneFolder` hint, `core.lua` reads 1.0.0, `list` shows `OneFolder  1.0.0  16001
  pinned at 1.0.0`, the cache holds both zips; `--json rollback` is one document going forward to 1.1.0;
  the murloc version prints two two-line messages. `rollback` of an unknown name, of a folder installed by
  hand, of an addon with no previous version, and with the kept zip deleted (in `--json`, the error document
  on stderr and nothing on stdout) exit 1 with their lines, and the sidecar and the files stay as they were.
- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90
  check --all-targets --locked` and `python -I scripts/changelog.py --check`: all green. No manifest
  changed, no new crate. No round on a real game folder: the tests run in temporary folders.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
