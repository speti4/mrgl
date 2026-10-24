---
status: idea
size: S
origin: done/120-a-usage-guide-as-html-inside-the-release-zip.md
created: 2026-10-24
milestone: M5 Trust
changelog: housekeeping
---

# Tests of the usage guide generator's error paths

What: unit tests of `scripts/usage-guide.py`'s rejections (a marker out of place, Markdown the page cannot
render, a link to a missing file, a render that is not deterministic) over a fixture README, run where the
`--check` runs in the `lint` job; the repo's first Python test file, in a shape its other scripts can follow.
Why: 120 verified those rejections by hand once; the generator runs in CI and in `release.ps1`, and a silent
change to a rejection would reach the zip.
Done when: each documented rejection has a test that fails without it, the tests run in CI, and
CONTRIBUTING's checks name the command.
