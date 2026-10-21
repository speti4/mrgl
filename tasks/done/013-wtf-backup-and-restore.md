---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
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

1. [ ] `backup.rs`: the zip writer over a `WTF` tree (relative paths, timestamps kept), the dated name, the
       list of backups
2. [ ] The restore: entry path check, the current `WTF` moved aside, the copy in, the revert on failure;
       the 023 guard before the first write
3. [ ] A synthetic fixture `WTF` tree (accounts, characters, `SavedVariables`, `AddOns.txt`); round-trip
       and escape tests; the refusal test with the stand-in client of 023
4. [ ] `mrgl-cli`: `backup` with `--to`, `--list`, `--restore`, `--yes`, the voice lines, `--json`,
       end-to-end tests
5. [ ] Docs; every check green; close

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
