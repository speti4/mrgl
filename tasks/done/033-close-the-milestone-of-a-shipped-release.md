---
size: S
origin: done/016-task-to-github-issue-mirror.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M2 Window MVP
changelog: housekeeping
---

# 033 — Close the GitHub milestone a release ships: a step in the release checklist

## Context

A step in the release checklist of `docs/development.md` (and `scripts/release.ps1` if it has a
checklist) that closes the GitHub milestone whose tasks the release ships; the mirror of 016 creates the
milestones from the roadmap headings but never closes one. An open milestone with every issue closed
misleads a contributor about where the project stands.
Done when: the checklist names the step with the `gh api` call, and the first release (012) closes its
milestone by it.

Decided 2026-10-22: paired with 061 on one branch; the step is documentation and, if `release.ps1`
carries a checklist, a prompt in it; the first use is 012's and is named there as the verification.

## Docs that move

- `docs/development.md` ("The release"): the step with the `gh api` call.

## Not in this change

- The release itself (012); a script that closes the milestone unattended.

## Steps

1. [ ] The step in the release checklist with the exact `gh api` call (find the milestone by title,
       patch `state: closed`)
2. [ ] `scripts/release.ps1`: the checklist prompt if the script has one, otherwise `none` in the Log
3. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), paired
  with 061

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
