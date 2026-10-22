---
size: S
origin: done/075-cli-output-survives-a-closed-pipe.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 084 — Stale staging folders are swept, so a run that ends mid-install leaves nothing behind

## Context

The install engine removes the download and staging folders of earlier runs under `<flavor-id>\staging\`
before it stages (a folder no running mrgl holds, by age or by a lock file), and the command line lets the
addon in flight finish before it exits on a closed pipe (075) so the sweep has less to do.
Why: a closed pipe at an install's `downloading` or `installing` line ends the process before the
`RemoveOnDrop` guard runs, and the download folder stays under `staging\` forever.
Done when: a planted stale staging folder is gone after the next install of the fixture flavor, the folder
of the run itself is untouched while it runs, and the closed-pipe test of 075 leaves no staging folder.

## Docs that move

- `docs/development.md`: the staging folder's lifetime (the sweep's rule, what counts as stale) and the
  closed-pipe rule of 075
- `CLAUDE.md`: the `staging\` line under "Data outside the repo", only if the layout changes (a lock file)

## Not in this change

- The zip cache under `cache\` and its housekeeping (072); a sweep of another flavor's staging folder; a
  sweep from the window

## Steps

1. [ ] `crates/mrgl-core/src/install.rs`: the engine sweeps the stale folders under `<flavor-id>\staging\`
       before it stages a run (the rule tells a folder of a running mrgl from a leftover: a lock file or an
       age, decided in the task and written into the Log); the run's own folder is never touched; a failed
       removal is a warning, never an error
2. [ ] `crates/mrgl-cli/src/main.rs`: on a closed pipe during an install the addon in flight finishes
       (the swap and the sidecar write), then the process exits quietly as 075 does
3. [ ] Tests: a planted stale folder is gone after the next fixture install, the run's own folder stays while
       it runs, and the closed-pipe test of 075 leaves no staging folder (`tests/install.rs`, `tests/cli.rs`)
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
