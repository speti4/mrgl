---
status: ready
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M1 Core + CLI
---

# Mirror the task files into GitHub issues and milestones

What: adopt the task standard's `mirror` (`scripts/sync-issues.py` is already in the repo for the changelog
generator; the workflow `sync-issues.yml` and the backfill are missing), one way, on every push to `main`
that touches `tasks/`.
Why: a public project's contributors look at issues; the task files stay the only place to edit.
Done when: every task and decision file has its issue, the milestones match the roadmap, and a second run
reports nothing to change.
