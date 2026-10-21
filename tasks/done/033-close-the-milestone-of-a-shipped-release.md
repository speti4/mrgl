---
size: S
origin: done/016-task-to-github-issue-mirror.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M2 Window MVP
changelog: housekeeping
closed: 2026-10-22
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

1. [x] The step in the release checklist with the exact `gh api` call (find the milestone by title,
       patch `state: closed`)
2. [x] `scripts/release.ps1`: the checklist prompt if the script has one, otherwise `none` in the Log
3. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), paired
  with 061
- 2026-10-22 — the step is item 4 of the release list in `docs/development.md` ("The changelog"): once the
  release is published, find the milestone by its title (the roadmap heading) and close it by its number
  when its open count is 0; a task of that milestone the release does not ship moves to a later milestone
  in its task file first, and a release that completes two milestones closes both. The two calls:
  `gh api "repos/speti4/mrgl/milestones?state=open&per_page=100" --jq '.[] | select(.title == "…") | …'`
  prints the number, title and counts; `gh api -X PATCH repos/speti4/mrgl/milestones/<number> -f
  state=closed` closes it. "The release" points to the list, and "The issue mirror" says that a release
  closes a milestone by hand, step 4.
- 2026-10-22 — `scripts/release.ps1`: none. The script has no checklist and no prompt; it builds and packs,
  and CI runs it unattended on the tag, where a prompt could not be answered.

## Deviations from the plan

- The step sits in the numbered release list under "The changelog", not in "The release": that list is the
  release checklist of `docs/development.md`, and "The release" has none; "The release" got a sentence that
  points to it.

## Verification

- 2026-10-22 — the lookup of the step run read-only against the repo in PowerShell, exactly as written:
  it printed `2  M2 Window MVP  open 9, closed 7`, the number the `PATCH` call takes. The `PATCH` was not run:
  no milestone is complete yet, and closing one is the release's step. Locally: `cargo fmt --check`,
  `python -I scripts/changelog.py --check` and `uvx zizmor .github` pass; no Rust code changed.
- The first use is the first release (012): its milestone is closed by this step once `v0.1.0` is
  published, and that close is the last part of this task's *Done when*.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
