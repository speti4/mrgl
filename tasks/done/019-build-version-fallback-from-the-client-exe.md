---
size: S
origin: wip/002-flavor-detection-and-addons-scan-with-toc-parsing.md
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 019 — Build version fallback from the client exe when `.build.info` has no row for the flavor

## Context

On a real install the Classic Era row was missing from `.build.info` while the client exe carried `1.15.7`;
the TOC chain of Era does not need the version, but the interface-versus-build warning of task 015 does.
When the install root's `.build.info` has no row for a flavor's product, `flavor.rs` reads the version
from the client exe's version resource (`Wow.exe` or `WowClassic.exe` in the flavor folder, the `FileVersion`
of its `VERSIONINFO`), so `Flavor.version` and the interface number are known; `.build.info` stays the first
source, and a flavor with neither still reports an unknown build with the warning 002 gives today.
Done when: a flavor folder with a `.build.info` row missing reports the exe's version with no warning, and a
fixture exe stub (a minimal PE with a version resource, synthetic) covers the parser offline.

Decided 2026-10-21: the PE version resource is read by a small parser of our own (the DOS header, the PE
header, the resource section, `VS_VERSIONINFO`), not through a Windows API, so the fixture test runs on
every target and no crate joins for a few hundred bytes of parsing.

## Docs that move

- `docs/development.md` ("Fixtures"): the synthetic exe stub and how it is generated.
- `crates/mrgl-core/tests/fixtures/README.md`: the stub.

## Not in this change

- The interface-versus-build warning itself (015).
- Any exe name list beyond `Wow.exe` and `WowClassic.exe`; an unknown name in a flavor folder is not read.

## Steps

1. [x] The PE version-resource parser in `flavor.rs` (or a `pe` submodule) with unit tests on byte slices
2. [x] The fallback in the flavor reader: `.build.info` row first, the exe second, the warning last
3. [x] A synthetic PE stub under `tests/fixtures/` written by a small generator script (no real exe), and
       the integration test over a fixture flavor folder without a `.build.info` row
4. [x] `scan`/`list` output and `--json` say where the version came from; docs; every check green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `crates/mrgl-core/src/pe.rs`: `read_file_version(path)` and `file_version(reader)` over
  any `Read + Seek`, reading only the bytes the walk touches (the DOS header, the PE and COFF headers, the
  optional header's resource data directory for PE32 and PE32+, the section table to map an RVA to a file
  offset, the resource directory tree down to `RT_VERSION` → the first name → the first language, the data
  entry, the `VS_VERSIONINFO` block). The version is the `FileVersion` string of the first `StringFileInfo`
  table, and the `dwFileVersion` of `VS_FIXEDFILEINFO` when the string is missing or not a version: the
  fixed info keeps 16-bit parts, so a build number above 65535 survives only in the string. Every failure is
  a plain reason, never a panic; a text file named like the client is "not an executable", not "truncated".
- 2026-10-21 — `flavor.rs`: `VersionSource` (`BuildInfo` | `ClientExe(name)`, with `kind()` and
  `file_name()`), `Flavor::version_source` (`None` exactly when the version is unknown), `CLIENT_EXES` and
  `client_exe_version(folder)`. `read_flavor` keeps the `.build.info` row first; on any failure (no row, no
  file, unreadable) it tries the exes; with no exe the warning is the one 002 gave, with an unreadable exe
  the warning names it: `build unknown: no row for wow_classic in .build.info; WowClassic.exe has no
  readable version (no version resource (RT_VERSION))`. The JSON of a flavor gains `version_source`
  (`build-info` | `client-exe` | null) and `version_file` (`.build.info` | the exe's name | null).
- 2026-10-21 — The fixture root `crates/mrgl-core/tests/fixtures/client-exe/`: a `.build.info` with a row
  for `wow` only and four flavor folders, the stubs written by `make-stubs.py` next to them (deterministic,
  1 to 1.5 KiB each, one `.rsrc` section and no code). `scan.rs` covers the fallback, the precedence of the
  row, the warning with and without an exe; `pe.rs` covers the parser over the stubs and bent copies of
  them (no MZ, no PE signature, truncated, no resource section, no version resource, a bad fixed info
  signature, a block that is not `VS_VERSIONINFO`, garbage) and the string forms (`1, 15, 7, 61582`,
  a `(build notes)` suffix).
- 2026-10-21 — `mrgl scan` / `mrgl list`: the summary says `, version read from WowClassic.exe` when the
  exe served and nothing when the row did; `--json` carries the two new fields of the flavor. The docs
  under *Docs that move* describe the fixture root and the generator.

## Deviations from the plan

- The parser is a module of its own, `mrgl_core::pe`, not a part of `flavor.rs`: it knows nothing about
  flavors, and `flavor.rs` is the one caller.
- The fixture is a second synthetic game folder (`client-exe/`), not a change to `flavors/`: removing a row
  from the shared `.build.info` would have moved the assertions of every test over that tree, in this and
  in the parallel branches.
- The version string is read before the fixed info (the Context names the `FileVersion`; the fixed info's
  16-bit parts cannot hold a build number above 65535, which the era stub carries on purpose).

## Verification

- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (mrgl-core: 47 unit tests of which 8 in `pe`, 13 in `tests/scan.rs` of which 3 new; mrgl-cli:
  25 passed and the stand-in ignored, 1 new end-to-end test over the `client-exe/` root for `scan`, `list`
  and `--json`), `cargo +1.90 check --all-targets --locked`, `python -I scripts/changelog.py --check`: all
  green in the worktree.
- 2026-10-21 — `python -I crates/mrgl-core/tests/fixtures/client-exe/make-stubs.py` run twice writes the same
  bytes (the same SHA-256 for every stub after the second run).
- 2026-10-21 — Against a real PE that is not a game file: a copy of Windows' `notepad.exe` as `Wow.exe` in a
  scratch flavor folder with a `.flavor.info` and no `.build.info`; `mrgl --serious scan` printed `no addons
  in _retail_ (wow 10.0.26100.9278, version read from Wow.exe)` and `--json` gave `version_source:
  "client-exe"`, `version_file: "Wow.exe"`; the number matches the `FileVersion` string in that file's
  resource. A manual round on the real install whose Classic Era row is missing is the maintainer's, with
  the sessions' one-at-a-time rule for the real game folders.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
