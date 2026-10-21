---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 004 — Install engine: zip to staging, addon-folder detection, atomic swap, rollback, the state sidecar; `add <zip>` and `remove`

## Context

The write path of the manager, the one place that touches the user's folder, so it has to be atomic and
bounded. `install.rs` unpacks a zip into a staging folder under `MRGL_HOME\<flavor-id>\staging\` (rejecting
entries that escape it), finds the addon folders in it (several per zip, each with its TOC), swaps them
into `Interface\AddOns` one folder at a time with a revert when a swap fails, keeps the previous version's
zip in `MRGL_HOME\<flavor-id>\cache\<addon>\<version>.zip` (one previous version; the rollback command of
022 reinstalls it; the cache is never required), and records the install in the state sidecar of D-003
(`state.json` per flavor: source, version, file list with hashes, install time); a missing or corrupt
sidecar is rebuilt from the scan. The CLI installs a zip in hand, the fourth source of D-002, with
`mrgl add <file.zip> --flavor <id>`, and removes a managed addon with `mrgl remove <name>`; `update <name>`
stays the stub that names 006 until the GitHub source (003) and the engine meet there.
Done when: the fixture zips (one folder, several folders, an escaping path) install, update and remove
correctly in a temporary AddOns folder, a failed swap leaves the folder as it was, and the sidecar
round-trips.

Decided 2026-10-21: the engine's entry takes a zip path and the install metadata (source, version), so the
GitHub source hands over a downloaded file and nothing else; the zip crate carries its one-line reason.

## Docs that move

- `crates/mrgl-core/Cargo.toml`: the zip and hashing crates join with their reasons; `deny.toml` if a
  license needs the allowlist.
- `docs/development.md` ("Layout", "Fixtures"): the `install` and `state` modules, the staging and cache
  folders under the mrgl home, the fixture zips.
- `README.md`: `mrgl add <file.zip> --flavor forever` and `mrgl remove <name>` as usage lines.
- `CLAUDE.md` ("Data outside the repo"): the cache next to the sidecar.
- `crates/mrgl-core/tests/fixtures/README.md`: the `zips/` fixtures.

## Not in this change

- The GitHub resolve and download (003) and `update` end to end (006).
- Rollback as a command (022), pin and ignore (021), junctions (005), export and import (015).
- The game-running warning on install (023 provides the check; the engine calls it once both are merged).

## Steps

1. [x] `state.rs`: the sidecar types, read, write, rebuild from the scan, `MRGL_HOME` layout per flavor
2. [x] `install.rs`: staging with the path check, addon-folder detection, the atomic swap with the revert,
       the kept previous zip, the sidecar record; `remove`
3. [x] Fixture zips (synthetic: one folder, several folders, an escaping path) and tests in a temporary
       AddOns folder, including the failed-swap case
4. [x] `mrgl-cli`: `add <file.zip> --flavor <id>`, `remove <name>`, the voice lines, `--json`, end-to-end
       tests
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `state.rs`: `FlavorHome` (`<home>\<flavor-id>\` with `state.json`, `staging\`,
  `cache\<addon>\<version>.zip`), `State` / `Entry` (schema 1: source spec, version, previous version,
  folders, `<Folder>/<path>` → SHA-256, RFC 3339 install time), `read_state_at` (missing → fresh; corrupt,
  another schema or another flavor → fresh with a warning), `write_state_at` through a temporary file,
  `reconcile` (an entry none of whose folders exists any more is dropped), `classify` (managed or
  unmanaged per scanned folder), `rebuild` (a fresh state over the scan: nothing in a folder says what mrgl
  installed, so a rebuilt state manages nothing until the next install). Six unit tests.
- 2026-10-21 — `install.rs`: `Engine { flavor, home }` with `stage` (unpack under `staging\<zip>-<nanos>-<n>\`,
  every entry through `safe_relative_path`: no `..`, no absolute path or drive, no reserved character or
  device name, `\` as a separator; a symbolic link entry refused; SHA-256 while writing), addon-folder
  detection (a TOC at the root makes the root one addon named after its base TOC; else every subfolder
  with a TOC of its own name; a single wrapper folder is looked into one level down), the name (the
  metadata's, else the folder the others extend, else the first) and the version (the metadata's, else the
  primary TOC's `## Version`, else `unknown`); `commit` (journaled moves: the old folder to
  `staging\…\previous\`, the staged one in, the folders the previous version had and this one lacks out;
  on a failure every move so far is undone in reverse and the error names the folder; a rename on one
  volume, a copy and a delete across volumes), the sidecar entry written, the zip copied to the cache and
  the cache pruned to the current and the previous version; `install` (read the sidecar, reconcile with a
  scan, stage, commit) and `remove` (the entry's folders, the entry, the cache; an unknown name says
  whether the folder exists). The place for the game-running check of 023 is marked in `commit`.
- 2026-10-21 — fixture zips under `tests/fixtures/zips/`, written by `make-zips.py` (deterministic: a
  rerun changes no byte), and `tests/install.rs` with 17 tests: install, update over the first version,
  the dropped folder of a later version, the kept and pruned cache, the TOC at the root, the wrapper
  folder, the metadata override, the escaping path (nothing written, staging cleaned), no addon, not a
  zip, the failed swap on the second folder (the first put back, the user's edit intact, the sidecar
  untouched), the failed first swap, the locked file on Windows, remove and its two errors, the hand-deleted
  folder, the corrupt sidecar, a folder taken over from another managed addon, the sidecar round trip.
- 2026-10-21 — `mrgl-cli`: `add <file.zip> --flavor <id> [--dry-run]` (a target that ends in `.zip` or is
  an existing file is a zip in hand; any other target stays the stub that names 003, so the `github:` form
  of 003 slots in next to it), `remove <name> --flavor <id>`, `update` names 006; the progress line, the
  warnings, the success line and the `--json` documents (`kind` `add` with `dry_run`, `kind` `remove`);
  seven end-to-end tests.
- 2026-10-21 — docs: `docs/development.md` (the engine in the layout, the staging and the cache under the
  home, the fixture zips), `README.md` (the two usage lines), `CLAUDE.md` (the cache next to the sidecar),
  the fixtures README; `Cargo.toml` with `zip` 9 (MIT; default features off, deflate on the pure Rust
  `zlib-rs` backend) and `sha2` 0.10 (MIT OR Apache-2.0), both on the allowlist, so `deny.toml` is unchanged.

## Deviations from the plan

- The name of an install is derived (the folder the others extend with `_` or `-`, else the first by
  name), or given in the metadata; the plan said "the zip's single folder or TOC", which covers one folder
  but not BigWigs-shaped zips. A single wrapper folder (a source archive) is looked into one level down.
- The cache keeps two zips per addon, the current and the previous version, since the previous one has to
  be there already when the next version arrives; "one previous version" holds.
- The fixture zips are committed (the plan allowed that or building them in the tests) and come from a
  deterministic generator next to them, so the escaping path is a crafted archive and not a writer trick.
- `remove` of a folder the sidecar does not know is refused with a message, never a delete: a folder by
  hand or by another manager is the user's.
- A cross-volume swap (the mrgl home on one drive, the game on another) is a copy and a delete rather than
  a rename; CI has one volume, so that path is for the manual round (a follow-up in the pull request).

## Verification

- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo test --locked` (mrgl-core: 40 unit, 17 install, 10 scan; mrgl-cli: 30 end to end; mrgl-app
  unchanged), `cargo +1.90 check --all-targets --locked`, `python -I scripts/changelog.py --check` and
  `cargo deny check` (advisories, bans, licenses, sources ok) all green in the worktree.
- 2026-10-21 — the fixture zips install, update (`one-folder` 1.0.0 → 1.1.0 → 1.2.0, `several-folders`
  2.0.0 → 2.1.0 with the `_Data` folder removed) and remove in a temporary AddOns folder; the escaping zip
  writes nothing; a swap that fails on the second folder leaves both folders, a file the user edited and
  the sidecar as they were; a file held open in the old folder fails the swap before anything moves; the
  sidecar round-trips byte for byte and a corrupt one is rebuilt with a warning. No real game folder and no
  network in any test; `MRGL_HOME` and `MRGL_ROOT` point every run at a temporary folder.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
