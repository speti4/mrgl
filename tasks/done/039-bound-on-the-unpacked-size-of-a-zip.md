---
size: S
origin: done/004-install-engine-with-state-sidecar.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M5 Trust
---

# 039 — A bound on the unpacked size of a zip

## Context

The install engine (004) stops unpacking when a zip's declared or actual unpacked size passes a
bound (a generous constant, overridable by a flag for a known large addon), reports it as the documented
error and leaves the staging folder clean. A zip bomb from a compromised source would fill the disk from
the staging folder; the hardening review of M5 asks for a bound on every untrusted input.
Done when: a synthetic zip that declares more than the bound is refused before the bound is written, and a
normal fixture zip is unaffected.

Decided 2026-10-22: the bound lives in the shared zip-entry check (044, `zip_entry.rs`), so the backup
restore (013) gets it too; the declared sizes are summed before anything is written, the actual bytes
counted while writing; the constant is 2 GiB and `--max-unpacked <size>` on `add`, `rollback` and
`backup --restore` (or the flag name the CLI prefers) overrides it.

## Docs that move

- `docs/development.md` (the shared zip-entry check and the engine's errors): the bound and the flag.

## Not in this change

- A bound on the download size (046, on the GitHub source branch); any other hardening item of M5.

## Steps

1. [x] The declared bound in `checked_entries` (sum of the entries' uncompressed sizes) and the actual
       bound while unpacking, one error for both naming the limit and the zip
2. [x] The flag on the CLI commands that unpack; the staging folder left clean after a refusal
3. [x] Tests: a synthetic zip declaring more than the bound refused before writing; one lying about its
       size refused while writing with the staging folder empty afterwards; a normal fixture unaffected;
       the restore path
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — the zip crate (9.0.5) read: `ZipArchive::by_index` already bounds each entry's output by
  its declared uncompressed size (its CRC reader fails with `InvalidData`, "File is larger than its
  declared uncompressed size", on the read that passes it), and deflate allocates nothing by the declared
  size. So a zip whose entries keep to their sizes can only pass the bound by declaring it, which the sum
  catches before anything is written; the counter while writing is the second line, and it also turns the
  crate's per-entry refusal into the documented error instead of a plain I/O failure.
- 2026-10-22 — `zip_entry.rs`: `MAX_UNPACKED` (2 GiB); `CheckedEntry` carries the declared `size` (0 for a
  folder); `checked_entries(archive, zip, role, max_unpacked)` sums the file entries' sizes (saturating)
  once every name passed and refuses a sum past the bound; `Unpacked::new(zip, role, limit)` and
  `Unpacked::entry(checked, reader)` give a counting reader that refuses once the zip's bytes pass the
  bound or the entry passes its declared size, and maps the crate's refusal to the same lie; the refusal
  travels inside the `io::Error` and `zip_entry::refusal` takes it back out. `Excess` (`Declared`,
  `Written`, `Entry`) is the clause; `size_text` writes sizes in steps of 1024 as the CLI's listings do,
  `parse_size` reads `3G`, `500M`, `64K`, `1048576`.
- 2026-10-22 — `error.rs`: `ZipTooLarge { role, path, excess }` appended, `refused <zip>: <clause>`.
- 2026-10-22 — `install.rs`: the engine carries `max_unpacked` (`MAX_UNPACKED` from `new` and `with_home`),
  `Engine::with_max_unpacked` sets it, `stage` hands it to `unpack`, and `unpack` reads every entry through
  the counter; a refusal leaves through `stage`'s `?`, so the drop of `Staged` removes the staging folder
  as after any failed stage. The rollback stages through the same call and gets the bound with no change of
  its own. `backup.rs`: `restore_wtf` calls the new `restore_wtf_bounded(flavor, zip, backups,
  max_unpacked)`; the declared refusal comes before `WTF` moves aside; a refusal while writing removes the
  half-written `WTF`, puts the aside one back and is `RestoreFailed` with the bound's clause as the reason.
- 2026-10-22 — `mrgl-cli`: `--max-unpacked <SIZE>` on `add` (a zip in hand and `github:`), `rollback` and
  `backup` (which requires `--restore` and conflicts with `--list`); a size that does not parse or zero is
  a usage error (exit 2).
- 2026-10-22 — fixtures: `make-zips.py` gains `declare_size`, which rewrites one entry's uncompressed size
  in its local header and its central directory entry and keeps its data and CRC; four new zips of 137 to
  558 bytes: `declares-too-much-1.0.0.zip` and `wtf-declares-too-much.zip` (3 GiB declared, a few bytes of
  data), `lies-about-its-size-1.0.0.zip` and `wtf-lies-about-its-size.zip` (16 bytes declared, about 64 KB
  of padding). The rerun leaves every existing fixture byte-equal.
- 2026-10-22 — docs/development.md: a new section, "The unpacked-size bound" (the bound, the flag, the two
  checks, the messages, the error, the fixtures).

## Deviations from the plan

- A refusal while the restore writes is reported as `RestoreFailed` with the bound's clause as its reason
  (`restoring <zip> failed and WTF is as it was: the entry … unpacks to more than …`), not as a bare
  `ZipTooLarge`: every failure after `WTF` moved aside says where `WTF` is, and the restore keeps that
  rule. The declared refusal, before anything moves, is `ZipTooLarge` for the engine and the restore alike.
- Besides the bound on the sum, an entry that unpacks to more than it declares is refused on its own (the
  `Entry` clause): the sum is only a bound while every entry keeps to its size, and the zip crate already
  enforces that per entry, so the lie is caught there first and reported as the documented error.
- The crate's per-entry refusal is recognized by its message text (`outgrew_its_declared_size`); a crate
  update that rewords it would turn the lie back into a plain I/O failure (still refused, the staging
  folder still removed), and the unit test over the lying fixture fails then.
- `restore_wtf` keeps its signature (the default bound) and `restore_wtf_bounded` is new, so the existing
  callers and tests stay as they are.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green. In `zip_entry`: two 10-byte files pass a bound of
  20 and are refused at 19 (`Declared { declared: 20, limit: 19 }`); `declares-too-much-1.0.0.zip` is
  refused at the default bound from its central directory with ``it declares 3.0 GB unpacked, more than
  the bound of 2.0 GB; nothing was written, and `--max-unpacked <size>` raises the bound for a zip you
  trust`` and passes under 4 GiB; the counter over a plain reader refuses an entry declaring 10 bytes at 38
  bytes written (`Entry`) and a second 30-byte entry under a bound of 50 at 46 (`Written`, `it unpacked
  past the bound of 50 B (46 B written, then removed)`); `lies-about-its-size-1.0.0.zip` read through the
  counter is refused at `LiesAboutItsSize/core.lua` with 109 B written (the TOC); `parse_size` and
  `size_text` over their cases.
- 2026-10-22 — `crates/mrgl-core/tests/install.rs`: `declares-too-much-1.0.0.zip` is refused by
  `Engine::install` with the declared line, no folder, an empty staging folder, no sidecar, then installs
  with `with_max_unpacked(4 GiB)`; `lies-about-its-size-1.0.0.zip` is refused with the `Entry` clause
  (`… so the bound of 2.0 GB cannot hold (109 B written, then removed)`), no folder, the staging folder
  empty, no sidecar; `one-folder-1.0.0.zip` is refused one byte under its declared size and installs at
  exactly it, and 1.1.0 installs at the default bound; a rollback under a bound of 100 bytes is refused
  naming the cached zip, the sidecar byte-equal, the addon's files unchanged, the staging folder empty.
- 2026-10-22 — `crates/mrgl-core/tests/backup.rs`: `wtf-declares-too-much.zip` is refused by
  `restore_wtf` (`ZipTooLarge`, role `Backup`) with `WTF` unchanged and no backups folder made; the fixture
  tree's own backup is refused by `restore_wtf_bounded` one byte under its size with nothing moved aside;
  the 3 GiB zip restores under 4 GiB (`Config.wtf` holds its 17 bytes); `wtf-lies-about-its-size.zip` fails
  with ``restoring <zip> failed and WTF is as it was: the entry `Config.wtf` unpacks to more than the 16 B
  it declares, so the bound of 2.0 GB cannot hold (0 B written, then removed)`` and `WTF` is byte-equal to
  before, no aside folder left.
- 2026-10-22 — end to end through the exe (`crates/mrgl-cli/tests/cli.rs`): `add
  declares-too-much-1.0.0.zip` exits 1 with the declared line, the dry run too, nothing installed, the
  staging folder empty, no sidecar; `--max-unpacked 4G` installs it; `--json add
  lies-about-its-size-1.0.0.zip` exits 1 with the `Entry` message in the JSON error document, the staging
  folder empty; `rollback OneFolder --max-unpacked 100` exits 1 (`more than the bound of 100 B`) with the
  sidecar and `core.lua` unchanged, `--max-unpacked 1M` rolls back; `backup --restore
  wtf-declares-too-much.zip --yes` exits 1 with nothing moved, `--max-unpacked 4G` restores it, the lying
  backup exits 1 with the `RestoreFailed` line and `WTF` as it was; `--max-unpacked lots`, `0`, `1.5G`,
  `backup --max-unpacked 3G` without `--restore` and with `--list` are usage errors (exit 2). The existing
  fixture tests (every other install, update, rollback and restore) pass unchanged at the default bound.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90
  check --all-targets --locked` and `python -I scripts/changelog.py --check`: all green. No manifest
  changed, so `cargo deny` and `zizmor` were not needed.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
