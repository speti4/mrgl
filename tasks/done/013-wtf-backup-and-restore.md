---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M3 Toolbox
---

# 013 — WTF backup and restore

## Context

The character and addon settings of the first weeks after launch are the most expensive thing to lose.
`backup.rs` copies a flavor's `WTF` folder into a dated zip (`mrgl backup --flavor <id> [--to <folder>]`,
the default folder under the mrgl home, `backups\<flavor-id>\WTF-<date>.zip`, listed by `mrgl backup
--list`), and `mrgl backup --restore <zip> --flavor <id>` puts one back after a confirmation (`--yes` for
scripts): every entry is checked to land under `WTF` and nowhere else, the current `WTF` is moved aside
into the same backups folder before the restore so a failed restore is reversible, and the restore refuses
while the client runs (the guard of 023). The window's two actions (backup, restore) build on the same
calls later (009/010 territory, not here). The first of the three `WTF` writes of D-002 and D-003.
Done when: a backup round-trips a fixture `WTF` folder byte for byte, a restore never writes outside `WTF`,
and a restore with the stand-in client running is refused with the documented line.

Decided 2026-10-21: a backup is a plain zip a user can open by hand; the manifest is the zip itself (no
sidecar entry), and the backups folder is never pruned by this task (a retention rule is a later one).

## Docs that move

- `docs/development.md` ("Layout", the mrgl home paragraph): `backups\` under the home folder.
- `README.md`: `mrgl backup --flavor forever` and the restore line.
- `CLAUDE.md` ("Data outside the repo"): the backups folder next to the sidecar.
- `crates/mrgl-core/tests/fixtures/README.md`: the fixture `WTF` tree.

## Not in this change

- Addon sets (014), the SavedVariables cleanup (025), a retention rule for old backups.
- The window's buttons (010 or a later window task).

## Steps

1. [x] `backup.rs`: the zip writer over a `WTF` tree (relative paths, timestamps kept), the dated name, the
       list of backups
2. [x] The restore: entry path check, the current `WTF` moved aside, the copy in, the revert on failure;
       the 023 guard before the first write
3. [x] A synthetic fixture `WTF` tree (accounts, characters, `SavedVariables`, `AddOns.txt`); round-trip
       and escape tests; the refusal test with the stand-in client of 023
4. [x] `mrgl-cli`: `backup` with `--to`, `--list`, `--restore`, `--yes`, the voice lines, `--json`,
       end-to-end tests
5. [x] Docs; every check green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `backup.rs`: `backup_wtf` walks `WTF` in name order into `WTF-<date>.zip` (written as
  `.zip.part`, renamed when complete; every file deflated with its modification time in the zip's own date
  fields as local time and exactly in the extended timestamp field `0x5455`; a folder is a directory entry,
  a link is a warning and left out), `list_backups` reads a backups folder (the zips by name with their file
  count, size and time, the aside folders), `restore_wtf` runs the 023 guard first, checks every entry
  (`safe_entry_path`: no `..`, absolute path, drive letter, reserved device name or control character; no
  symbolic link entry) before anything is written, moves the current `WTF` into
  `backups\<flavor-id>\WTF-before-restore-<date>` (one rename; a copy and a removal across volumes), unpacks,
  and on a failure removes the half-written folder and puts the aside one back. The dated names and the
  zip timestamps use the local time of the machine through the Windows SDK's time-zone conversion
  (`Win32_System_Time`, two calls on the `windows-sys` bindings already in the tree), UTC on the other
  targets until M6.
- 2026-10-21 — the `zip` crate joins the workspace with the same dependency block task 004 adds (default
  features off, deflate on the pure Rust `zlib-rs` backend; MIT); the merge resolves the duplicate.
- 2026-10-21 — `mrgl backup`: `--to <folder>` (the backups folder, by default `backups\<flavor-id>` under the
  mrgl home), `--list`, `--restore <zip>` (a path, or a bare name from `--list` looked up in the backups
  folder) with a confirmation read from the standard input, `--yes` for scripts and required with `--json`;
  the JSON documents `backup`, `backup-list` and `restore`. The fixture tree `tests/fixtures/wtf/WTF/`, the
  core tests in `tests/backup.rs`, the end-to-end tests in the CLI's `tests/cli.rs`.

## Deviations from the plan

- The confirmation of a restore is refused outright under `--json` without `--yes` (an error, exit 1)
  instead of being read from the standard input: a machine reader has nobody to answer it.
- `--restore` takes a bare name from `--list` as well as a path; the plan named a path only.
- The entry check is written in `backup.rs` rather than shared with the install engine's (task 004 lands
  on another branch at the same time); folding the two into one helper is a follow-up of the pull request.

## Verification

- 2026-10-21 — `cargo test --locked`: every test green. `tests/backup.rs` backs the fixture `WTF` tree (8
  files, 7 folders with an empty one, every file dated to its own second) up into a zip and restores it into
  a second flavor folder: the snapshot of paths, bytes and modification times is equal before and after.
  Four escaping zips written by the test (`../`, an absolute path, a drive letter, `NUL`) are refused with
  `refused <zip>: the entry … would land outside WTF` and the `WTF` folder is untouched, as it is for a text
  file named `.zip` and a missing zip. A zip whose second entry cannot be written (a file where a folder is
  wanted) fails with `restoring … failed and WTF is as it was` and the snapshot is unchanged, the aside
  folder gone. With the stand-in client of task 023 started from the flavor folder, the restore is refused
  with `the client is running in _forever_ (client-stand-in.exe, pid N): close the game first, nothing under
  WTF is written while it runs` and nothing is touched; a backup goes on. The same through the exe in
  `tests/cli.rs`: the two-line summary, the listing with its `1 backup of _forever_ in …` summary and the
  aside count after a restore, the confirmation prompt answered `n` (`restore cancelled, nothing changed`,
  exit 0), `--json --restore` without `--yes` (exit 1), the `backup`, `backup-list` and `restore` documents,
  the default folder under `MRGL_HOME`, and the refusal while the stand-in runs (exit 1, the documented
  line).
- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked`, `python -I scripts/changelog.py --check` and `cargo deny check` (the manifests
  changed): all green.
- No round on a real game folder in this change (the real `WTF` is a shared resource of the sessions); that
  round is a follow-up of the pull request.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
