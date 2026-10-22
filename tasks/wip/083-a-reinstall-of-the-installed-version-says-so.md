---
size: S
origin: done/077-marks-survive-a-reinstall-and-the-pinned-line-orders-versions.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 083 — A reinstall of the installed version says it was reinstalled, not updated from the previous version

## Context

When a zip in hand or a pick installs the version the sidecar already records, the install summary says
`reinstalled <name> <version>` instead of `updated <name> from <previous> to <same version>`, in the text
and as an `outcome` in the JSON; the entry's previous version stays what it was (077).
Why: `Installed::previous_version` carries the entry's older version, so a reinstall reads as an update
between two different versions that are the same.
Done when: a fixture reinstall of the installed version prints the reinstalled line and keeps the previous
version for `rollback`, and a real update still prints `updated … from … to …`.

Decided 2026-10-22: runs after 082 on the same branch, since both change the command line's install summary.

## Docs that move

- `docs/development.md`: the install summary's outcomes (`installed`, `updated`, `reinstalled`) and the JSON key
- `README.md`: only if an example shows the reinstall line

## Not in this change

- The commit rule of 077 (which entry a staged name replaces) and the previous version it keeps
- The window's wording of an update answer, unless it prints the same core line

## Steps

1. [ ] The core's install result tells a reinstall of the recorded version apart from an update between two
       versions (an outcome the summary and the JSON read), with the previous version kept as 077 keeps it
2. [ ] `mrgl-cli/src/main.rs`: the summary line `reinstalled <name> <version>` through the voice, the JSON
       `outcome`; `updated … from … to …` unchanged for a real update
3. [ ] Tests: a fixture reinstall of the installed version prints the reinstalled line and `rollback` still
       finds the previous version; a real update prints the updated line (`tests/install.rs`, `tests/cli.rs`)
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
