---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 025 — SavedVariables: size per addon and orphan cleanup into a backup zip; CLI `savedvars`

## Context

`mrgl savedvars --flavor <id>` lists the `SavedVariables\<Addon>.lua` sizes per account and per
character, and marks orphans (a file whose addon folder no longer exists under `Interface\AddOns`; a disabled
addon keeps its folder, so it is never an orphan). `--clean` moves the orphans and their `.bak` files into a
dated zip next to the backups of 013, never deletes, and refuses while the game runs (023). The third `WTF`
write of D-002 and D-003 (amended 2026-10-21). The classic `WTF` bloat: megabytes of settings for addons
removed months ago, and a 50 MB file behind a long loading screen.
Done when: a fixture `WTF` folder lists sizes and orphans correctly, `--clean` round-trips the moved files
through the zip byte for byte, and nothing outside `SavedVariables` is touched.

Decided 2026-10-22: the cleanup zip is `backups\<flavor-id>\SavedVariables-<date>.zip`, written with the
backup module's zip writer (013), and `backup --restore` is not taught to read it (a restore of orphans
is an unzip by hand, documented).

## Docs that move

- `README.md` ("What it will do") and `docs/development.md` ("Layout", the home folder's files): the
  command, the orphan rule and the zip's name.

## Not in this change

- A size bound or a warning on a large file in `list`; the window; a restore of the cleanup zip.

## Steps

1. [x] The SavedVariables walk in the core (account-wide and per-character files, sizes, the orphan rule
       against the AddOns scan)
2. [x] `--clean`: the orphans and their `.bak` files into the dated zip, moved not copied, the `WTF` write
       guard of 023 before the move
3. [x] The CLI command with the voice's lines and `--json`
4. [x] Tests on a fixture `WTF`: the sizes, the orphans (a disabled addon is not one), the byte-for-byte
       round trip through the zip, the refusal while the stand-in client runs
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — `savedvars.rs` (new module): `list_saved_variables` walks `WTF\Account\<account>\SavedVariables\`
  and `WTF\Account\<account>\<realm>\<character>\SavedVariables\` (links are a warning and not followed, a
  realm folder named `SavedVariables` is the account-wide one); a `<Addon>.lua` and its `<Addon>.lua.bak`
  are one entry (`lua_bytes`, `bak_bytes`, `bytes`), any other file is neither listed nor touched; the
  entries are sorted by size, the largest first, with the totals and the orphan count. Each entry has a
  `status`: `installed`, `orphan`, `client` (a `Blizzard_` file: the client's own interface has no folder
  under `Interface\AddOns`, so without this rule every one of them would be an orphan) or `unknown` (no
  AddOns folder at all: nothing is marked, and a warning says so).
- 2026-10-22 — the orphan rule compares the file's addon name with the folders of the AddOns scan of task
  002, lower-cased on both sides: a folder the scan reads as an addon (a TOC named after it) counts, and so
  does a folder the scan cannot read; a folder without a TOC does not count, so its file is an orphan.
  `AddOns.txt` is not read, so a disabled addon keeps its file.
- 2026-10-22 — `clean_orphans`: the guard of task 023 first, then the listing, then the orphans and their
  `.bak` files into `SavedVariables-<date>.zip` with entry names relative to `WTF`, then each file removed
  only once its entry, read back from the finished zip, compares equal byte for byte (a mismatch or a failed
  removal stops the rest: `Error::CleanupIncomplete` names the zip and how many files were removed). No
  orphan: nothing is written, not even the backups folder.
- 2026-10-22 — `backup.rs`: the zip writer of task 013 split into `dated_zip` (the fresh dated name, the
  `.part` file, the rename), `write_zip` and `add_file` (one deflated entry with its times); `backup_wtf`
  runs on them unchanged, and `zip_files` (crate-private) writes a given list of files, which the cleanup
  calls. `restore_wtf` refuses a zip named `SavedVariables-…` with `Error::NotAWtfBackup` before the
  client guard and before anything moves.
- 2026-10-22 — `mrgl savedvars --flavor <flavor> [--clean]`: a data line per entry (size, addon, `account`
  or `account / realm / character`, `orphan` or `client`), the warnings, and a summary through the voice
  (success with no orphan, warning with orphans, nothing for an empty `WTF`); `--clean` prints the moved
  entries and `moved N orphans of <flavor> (F files, size) into <zip>; unzip it into the WTF folder to put
  them back`, or `no orphans in the SavedVariables of <flavor>, nothing moved`; `--json` is the
  `savedvars` or the `savedvars-clean` document. No crate added.

## Deviations from the plan

- `backup --restore` refuses the cleanup zip with its own line instead of treating it as a `WTF` backup: a
  restore of it would have moved the whole `WTF` folder aside for a few files.
- The `Blizzard_` rule and the `unknown` status (no AddOns folder) were not in the plan; both keep the
  cleanup from moving files whose addon is not missing.
- `--clean` asks no confirmation: it moves files into a zip and deletes nothing, and the listing without
  `--clean` shows what it would move.
- The tests add their SavedVariables files and AddOns folder to a temporary copy of the fixture `WTF` tree
  of task 013 instead of a new fixture tree, so the backup tests' counts of that tree stay as they are.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green. `crates/mrgl-core/tests/savedvars.rs` copies the
  fixture `WTF` into a temporary flavor folder and adds made-up files: the listing has 8 entries with the
  statuses expected (`GoneAddon` `.lua` and `.bak`, `NoTocFolder` and a lone `GoneToo.lua.bak` orphans;
  `DisabledAddon`, disabled in `AddOns.txt`, and `mrglcase` against the `MrglCase` folder installed;
  `Blizzard_Console` the client's; `notes.txt` not listed), the sizes (300 + 200 = 500 for `GoneAddon`),
  the order and the totals (3 orphans, 710 bytes), and nothing under `WTF` changes. `--clean` writes one
  `SavedVariables-<date>.zip` whose entries are exactly the 4 orphan files under their `WTF` paths with the
  same bytes; afterwards `WTF` equals its snapshot minus those 4 files; written back from the zip, it equals
  the snapshot before. A cleanup with no orphan writes nothing; without an AddOns folder nothing is an orphan
  and the warning is there; `backup::restore_wtf` refuses the cleanup zip with `not a WTF backup: …` and
  nothing moves. With the stand-in client of task 023 running from the flavor folder, the cleanup is refused
  with `the client is running in _forever_ (client-stand-in.exe, pid N): close the game first, nothing under
  WTF is written while it runs`, nothing is touched and no zip is written; the listing still runs. The same
  through the exe in `crates/mrgl-cli/tests/cli.rs` (the data lines, the summary, the murloc line, both JSON
  documents, the zip under `MRGL_HOME`, the restore refusal and the refusal while the stand-in runs, exit 1).
- 2026-10-22 — one full `cargo test` run failed three tests that spawn the stand-in client (two of them
  written before this task) at `the stand-in starts`: the test binaries of this wave were quarantined by a
  Windows Defender false positive at the time. The rerun passed every test.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check`: green. No manifest changed, so no
  `cargo deny check`.
- No round on a real game folder in this change (the real `WTF` is a shared resource of the sessions); that
  round is a follow-up of the pull request.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
