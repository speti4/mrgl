---
size: S
origin: done/005-dev-addon-linking-with-junctions.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 061 — CI compiles the non-Windows paths of the core

## Context

A CI job on the Linux runner runs `cargo check --all-targets` (or the core's tests) so the
`cfg(not(windows))` branches of `link`, `client` and `discover` compile; today the Linux job only runs
`fmt` and the changelog check, and a broken non-Windows path is found at M6. The macOS preview (M6) starts
from a core that compiles there; cheap to keep true from now.
Done when: the job runs on every pull request and a deliberate non-Windows compile error fails it.

Decided 2026-10-22: paired with 033 on one branch (both are CI and release housekeeping); the check is
`cargo check --all-targets --locked` on `ubuntu-latest` for the workspace, the Tauri crate included only
if its Linux system dependencies install in under a minute, otherwise `-p mrgl-core -p mrgl-cli`.

## Docs that move

- `docs/development.md` ("CI and releases"): the new job and what it proves; `CONTRIBUTING.md` if its
  check list names the jobs.

## Not in this change

- A macOS job (036, M6); running the tests on Linux (the fixtures are Windows-shaped today).

## Steps

1. [ ] The job in `.github/workflows/ci.yml` in the house style (pinned actions, the toolchain action,
       the cache); `uvx zizmor .github` clean
2. [ ] The non-Windows paths fixed if the first run finds them broken (the fix is this task's)
3. [ ] The job added to the required checks is the main session's step after the merge; say so in
       Verification
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), paired
  with 033

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, at close)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
