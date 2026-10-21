---
status: idea
size: S
origin: done/016-task-to-github-issue-mirror.md
created: 2026-10-21
milestone: M2 Window MVP
---

# Close the GitHub milestone a release ships: a step in the release checklist

What: a step in the release checklist of `docs/development.md` (and `scripts/release.ps1` if it has a
checklist) that closes the GitHub milestone whose tasks the release ships; the mirror of 016 creates the
milestones from the roadmap headings but never closes one.
Why: an open milestone with every issue closed misleads a contributor about where the project stands.
Done when: the checklist names the step with the `gh api` call, and the first release (012) closes its
milestone by it.
