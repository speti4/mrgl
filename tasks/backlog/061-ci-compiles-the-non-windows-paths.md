---
status: idea
size: S
origin: done/005-dev-addon-linking-with-junctions.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# CI compiles the non-Windows paths of the core

What: a CI job on the Linux runner runs `cargo check --all-targets` (or the core's tests) so the
`cfg(not(windows))` branches of `link`, `client` and `discover` compile; today the Linux job only runs
`fmt` and the changelog check, and a broken non-Windows path is found at M6.
Why: the macOS preview (M6) starts from a core that compiles there; cheap to keep true from now.
Done when: the job runs on every pull request and a deliberate non-Windows compile error fails it.
