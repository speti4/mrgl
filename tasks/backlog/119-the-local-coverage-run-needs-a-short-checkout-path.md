---
status: idea
size: S
origin: done/118-one-stand-in-client-helper-for-the-tests-of-the-three-crates.md
created: 2026-10-24
milestone: M5 Trust
changelog: housekeeping
---

# The local coverage run needs a short checkout path

What: CONTRIBUTING's coverage paragraph says that the local measurement runs from a short checkout path (a
`subst` drive or a short folder), and names the symptom of a long one: the window crate's rustc command line
exceeds the 32,767 characters Windows allows, cargo hands rustc an argument file, cargo-llvm-cov's wrapper
then misses `--crate-name` and does not instrument the crate, and the run reports `mrgl-app` missing and core
lines missed.
Why: a checkout under a long folder (a git worktree with a long branch name) measures a wrong result without
an error; CI is not affected, so the local run is the only place it bites.
Done when: the paragraph is in CONTRIBUTING.md and a run from a `subst` drive of the repo reaches the same
numbers as the `coverage` job.
