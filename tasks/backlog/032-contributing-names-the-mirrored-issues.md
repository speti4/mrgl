---
status: idea
size: S
origin: done/016-task-to-github-issue-mirror.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# CONTRIBUTING.md names the mirrored issues as the task files' public face

What: one line under "Start with an issue" and "Tasks and decisions" in `CONTRIBUTING.md`: the `mirrored`
issues are the task files' public face, comments on them are welcome, and the file under `tasks/` is what
the maintainer edits (016 installed the mirror; the contributor guide does not say so yet).
Why: a contributor who comments on a mirrored issue should know where the text comes from and that the
issue body is overwritten by the next sync.
Done when: the two sections carry the line, and `tasks/README.md`, `CLAUDE.md` and `CONTRIBUTING.md` tell
the same story.
