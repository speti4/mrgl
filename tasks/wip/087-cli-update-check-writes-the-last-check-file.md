---
size: S
origin: done/078-the-windows-cached-view-walks-the-releases-and-keeps-the-last-answer.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 087 — The command line's update check writes the last-check file the window reads

## Context

`mrgl update --all` and `mrgl update <name>` record their check in `<flavor-id>\last-check.json` through
`mrgl_core::last_check` as the window's check does (078), so a source gone, a failed check or a budget stop
found on the command line shows in the window, and a source found back clears it there.
Why: the file is written by the window only, so after `mrgl update` finds a gone source back while the
version stays the same, the window still shows it gone until its own next check.
Done when: a CLI test's check writes the file with the kept states, a second check that finds the source
back clears the entry, and the window's flavor list test reads what the command line wrote.

Decided 2026-10-22: runs last on the branch of 082 and 083, since it changes the same `update` command.

## Docs that move

- `docs/development.md`: the last-check file's writers (the window and the command line) and the `update` command
- `README.md`: the kept answers paragraph, if it says the window alone writes them

## Not in this change

- The window's check and the rules of `last_check` (078); a new kept state

## Steps

1. [ ] `mrgl-cli/src/main.rs`: `update --all` and `update <name>` write the kept states of their check through
       `mrgl_core::last_check` after the run, under the same rules as the window (a failed write is a warning)
2. [ ] Tests: a CLI test's check writes the file with the kept states and a second check that finds the source
       back clears the entry (`tests/cli.rs`); the window's flavor list test reads a file the command line's
       shape wrote (`crates/mrgl-app/tests/flavor_list.rs`)
3. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

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
