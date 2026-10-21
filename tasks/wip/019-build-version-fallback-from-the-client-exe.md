---
size: S
origin: wip/002-flavor-detection-and-addons-scan-with-toc-parsing.md
created: 2026-10-21
started: 2026-10-21
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

1. [ ] The PE version-resource parser in `flavor.rs` (or a `pe` submodule) with unit tests on byte slices
2. [ ] The fallback in the flavor reader: `.build.info` row first, the exe second, the warning last
3. [ ] A synthetic PE stub under `tests/fixtures/` written by a small generator script (no real exe), and
       the integration test over a fixture flavor folder without a `.build.info` row
4. [ ] `scan`/`list` output and `--json` say where the version came from; docs; every check green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, when the work is done)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the stale index is the expected finding on a task branch)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
