---
status: idea
size: S
origin: done/040-add-github-installs-the-pick-through-the-engine.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# One install summary helper for `add <file.zip>` and `add github:`

What: the install summary line ("installed … into", "updated … from … to …") and its JSON block are
written twice in the CLI, once for the zip in hand and once for the pick; fold them into one helper
that both call, once 037 and 038 are on `main` (the managed view and the running-client warning touch
the same lines).
Why: two copies of the user-facing result drift apart, and `update --all` (006) and `import` (015) want
the same line a third and fourth time.
Done when: one helper, every install path calls it, the CLI tests of both paths unchanged.
