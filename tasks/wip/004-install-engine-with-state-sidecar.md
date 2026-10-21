---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
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

1. [ ] `state.rs`: the sidecar types, read, write, rebuild from the scan, `MRGL_HOME` layout per flavor
2. [ ] `install.rs`: staging with the path check, addon-folder detection, the atomic swap with the revert,
       the kept previous zip, the sidecar record; `remove`
3. [ ] Fixture zips (synthetic: one folder, several folders, an escaping path) and tests in a temporary
       AddOns folder, including the failed-swap case
4. [ ] `mrgl-cli`: `add <file.zip> --flavor <id>`, `remove <name>`, the voice lines, `--json`, end-to-end
       tests
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

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
