---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 029 — Dependency check from the TOC `## Dependencies` and `## RequiredDeps` and the `release.json` `nolib` flag

## Context

Read `## Dependencies` and `## RequiredDeps` from each installed addon's TOC and the `nolib` flag of the
source's `release.json`, and report a missing required dependency after an install and in `list`; an
embedded library (a `nolib` build not chosen) is not missing. A missing library is the second most common
"it does not load" after a wrong flavor pick.
Done when: fixture addons with met, missing and embedded dependencies report as documented, offline.

Decided 2026-10-21: the TOC parser already collects the required and optional dependency names (002);
this task adds the check over a scan: a required name is met when a folder of that name is in the scan
or is a subfolder of the addon itself (an embedded library, which is what a non-`nolib` build carries),
missing otherwise; an optional dependency is never reported; the pick's `nolib` flag (003) is recorded
on the sidecar entry so the check knows an embedded library is expected; the report is a warning line
per addon with the missing names, in `list` and after an install, never a refusal.

## Docs that move

- `docs/development.md` ("Layout"): the dependency check and what met, missing and embedded mean.
- `README.md`: the missing-dependency line a user may see.

## Not in this change

- Installing a missing dependency from a source (a later task, once the sources know library repos);
  the window's row (010); optional dependencies.

## Steps

1. [x] The check over a scan: met, missing and embedded per addon from the TOC's required names and the
       addon's own subfolders
2. [x] The sidecar entry records the pick's `nolib` flag (optional in the JSON; absent means unknown)
3. [x] `mrgl-cli`: the warning line in `list` and after `add` (plain through the voice, and `--json`
       fields per addon)
4. [x] Tests: fixture addons with a met, a missing and an embedded dependency, offline
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `mrgl_core::dependencies`: `check(scan, state)` returns an `AddonDependencies` per addon
  (`folder`, `required`, `met`, `embedded`, `missing`, `nolib`) from the required names of the TOC the
  flavor loads, each once, ASCII case-insensitive; `check_with` takes the subfolder lookup as a closure and
  is what the unit tests drive, `check` reads the folders inside an addon from disk (at most three levels,
  `EMBED_DEPTH`) and only when a name is not an addon of the scan. `AddonDependencies::warning` is the plain
  line, with a note for a `nolib` build.
- 2026-10-21 — the `nolib` flag travels from the pick to the sidecar: `Chosen.nolib` and `Pick.nolib`
  (`release.json`'s flag, or the `-nolib` name in a pick by name), `install::Metadata.nolib` and
  `Staged.nolib` (`Option<bool>`), and `state::Entry.nolib` at the end of the entry,
  `#[serde(default, skip_serializing_if = "Option::is_none")]`, so a sidecar written before reads as
  unknown and an unknown flag is not written.
- 2026-10-21 — `mrgl list` runs the check against the flavor's sidecar (an unreadable sidecar is a warning
  and leaves every flag unknown): a warning line through the voice per addon with a missing dependency,
  `missing: <names>` at the end of the addon's line and a `dependencies` object per addon in `--json`, both
  through one helper (`mark_dependencies`). `mrgl add <file.zip>` checks the installed folders after the
  install and adds the warning lines and `addon.dependencies` in `--json`. `mrgl scan` reads the folder only
  and runs no check.
- 2026-10-21 — fixtures: the game folder `tests/fixtures/dependencies/` (`Needy` with a met, an embedded and
  a missing dependency and an optional one, `LibMet`, `NolibBuild` with a library folder inside) and the zip
  `needs-library-1.0.0.zip` from `make-zips.py` (a rerun leaves every other zip byte for byte as it was).

## Deviations from the plan

- `scan` runs no check: it reads the folder only, without the sidecar, and its output stays as it was; the
  check is in `list` and after `add <file.zip>`. `add --dry-run` reports no dependencies (nothing is
  installed yet).
- Nothing installs from a pick yet (task 040), so the `nolib` flag reaches a sidecar only through
  `Metadata.nolib`, which the engine's tests set; a zip in hand records none (unknown). The install from a
  pick passes `Some(pick.nolib)` and calls the same after-install check, a follow-up of the pull request.
- Only the TOC the flavor loads is read: a folder the flavor loads no TOC of reports no dependency.
- The embedded lookup goes three folder levels down (`Libs\LibStub` is two), a bound the decision did not
  name.

## Verification

- 2026-10-21 — `cargo test --locked`: every test green. The unit tests of `dependencies.rs`: `Needy` with
  `## Dependencies: LibMet, LibEmbedded, LibMissing`, `## RequiredDeps: libmet` and `## OptionalDeps:
  LibOptional` reports `required` `LibMet, LibEmbedded, LibMissing`, `met` `LibMet`, `embedded`
  `LibEmbedded`, `missing` `LibMissing` and the line `Needy needs the missing dependency LibMissing: install
  it or the addon does not load`; an optional dependency alone reports nothing; a folder `libmet` meets
  `LIBMET`; with `nolib: true` on the entry `LibEmbedded` is missing and the line ends with `(a nolib build:
  its libraries are addons of their own)`, with `nolib: false` it is embedded; the subfolders are read once
  per addon and only when the scan does not answer; the walk stops at its depth. `tests/dependencies.rs` the
  same over the fixture folder on disk, and `NolibBuild` turning from embedded to missing with a `nolib`
  entry. The sidecar round-trips `nolib: true` and leaves an unknown flag out; the engine records
  `Metadata.nolib` (`Some(false)`) and nothing for a zip in hand; `Pick.nolib` is set by both pickers.
- 2026-10-21 — through the exe (`crates/mrgl-cli/tests/cli.rs`): `mrgl --serious list --flavor
  <fixture>\_forever_` prints `Needy  1.0.0  16001  missing: LibMissing`, the warning line and `3 addons in
  _forever_ (wowf 1.60.1.70245)`, exit 0; `--json` has `addons[].dependencies` as documented and the line in
  `warnings`; a sidecar under `MRGL_HOME` with `"nolib": true` for `NolibBuild` adds `missing:
  LibEmbedded` and the nolib line; `scan --json` has no `dependencies` field; `mrgl --serious add
  needs-library-1.0.0.zip --flavor forever` installs (exit 0) and prints the warning line before `installed
  NeedsLibrary 1.0.0 (1 folder: NeedsLibrary) into _forever_`, and `--json` has `addon.dependencies[0]` with
  `embedded` `LibEmbedded`, `missing` `LibMissing` and `nolib` `null`.
- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check`: all green. No manifest changed.
- No round on a real game folder in this change (the real folders are a shared resource of the sessions).

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
