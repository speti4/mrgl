---
size: S
origin: done/013-wtf-backup-and-restore.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M3 Toolbox
changelog: housekeeping
closed: 2026-10-21
---

# 044 — One zip-entry check for the install engine and the backup restore

## Context

The install engine's path check (004, `safe_relative_path`) and the restore's (013, `safe_entry_path`)
fold into one function with one `Error` pair (an unreadable zip, an escaping entry) that both call; the
two fixture tests stay and run against the shared check. Two copies of the one rule that keeps a zip
from writing outside its folder drift apart, and the hardening review of M5 wants one place to read.
Done when: one function, both callers, both escape tests green, and the duplicated error variants gone.

Decided 2026-10-21: the shared check lives in its own small module (`zip_entry.rs` or the name the
layout prefers), the stricter of the two rules wins wherever they differ (the difference is documented in
the module doc), and the user-facing messages of both commands stay as they are.

## Docs that move

- `docs/development.md` ("Layout"): the shared module, named by both the engine and the backup.

## Not in this change

- The bound on the unpacked size of a zip (039); any change to the engine's swap or the restore's
  moved-aside folder.

## Steps

1. [x] The shared function and its error pair in one module; the two callers switched to it
2. [x] The duplicated error variants removed; every message the two commands print unchanged (the CLI
       tests prove it)
3. [x] The two fixture escape tests run against the shared check, plus one unit test per rule difference
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — the two copies compared: `safe_relative_path` (engine) and `safe_entry_path` (restore) were
  the same name rule, word for word, and both callers refused a symbolic link. The callers differed twice:
  the engine checked each entry as it unpacked it (the escape fixture's TOC reached staging before the
  refusal), the restore checked every entry first; the engine opened each entry for reading (`by_index`),
  the restore only the raw entry (`by_index_raw`), so an encrypted entry or an unread compression method
  passed the restore's check and failed half way, after `WTF` had moved aside. The stricter side won both.
- 2026-10-21 — `crates/mrgl-core/src/zip_entry.rs`: `checked_entries(archive, zip, role) ->
  Result<Vec<CheckedEntry>>` opens every entry for reading, checks its name and refuses a link before it
  returns anything; the name rule (`entry_path`) and `is_reserved_device_name` are private to it, and the
  module doc states the rule and the two differences. `ZipRole` (`Addon`, `Backup`) names the zip
  (`zip`, `backup`) and the folder (`the staging folder`, `WTF`). `install::unpack` and the restore's
  `checked_entries` call it and write only what it hands back; `safe_relative_path`, `safe_entry_path`, the
  two copies of `is_reserved_device_name` and the restore's own `CheckedEntry` are gone.
- 2026-10-21 — `Error::ZipUnreadable` and `Error::ZipEntryEscapes` carry the role and render it in their
  `Display`; `BackupUnreadable` and `BackupEntryEscapes` are removed in place. The messages stay as they
  were: `not a readable zip` / `not a readable backup`, `would land outside the staging folder` / `would
  land outside WTF`.
- 2026-10-21 — tests: in `zip_entry`, the merged name rule (the cases of both former tests), the order of
  the returned entries, one test per rule difference (an escaping entry after good ones returns nothing; an
  encrypted entry makes a backup unreadable at the check) and the four messages by role, and a symbolic link
  entry; in `install`, the escape fixture unpacks nothing (its TOC no longer reaches staging); in
  `tests/backup.rs`, the escape test matches the shared variants and an encrypted backup is refused before
  anything moves aside. The fixture escape tests of both (`escaping-path.zip`, the four hand-written
  backups) run unchanged against the shared check.

## Deviations from the plan

- The shared function checks a whole open zip (`checked_entries`) rather than one name: the order and the
  entry opening were where the callers differed, so both had to move into the one place for the stricter
  rule to hold for both. Opening the file stays with each caller, because a missing backup keeps its own
  `backup not found` message and a missing addon zip stays `not a readable zip`.
- The merged variants keep the messages through a `role` field and the `Display` of each variant, as the
  task allowed; a caller that builds `ZipUnreadable` or `ZipEntryEscapes` now names its role.

## Verification

- 2026-10-21 — in the worktree: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo test --locked` (core unit tests 94, 89 before; `tests/install.rs` 17 with the escape fixture test
  unchanged; `tests/backup.rs` 8 with the encrypted case; `tests/cli.rs` 51 with the escape message of
  `add` unchanged and the restore lines unchanged), `cargo +1.90 check --all-targets --locked`,
  `cargo doc -p mrgl-core --no-deps --document-private-items` with `RUSTDOCFLAGS=-D warnings` and
  `python -I scripts/changelog.py --check`, all green. No manual round: the change touches no game folder
  behavior beyond the refusals the tests cover.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
