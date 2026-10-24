---
size: S
origin: done/120-a-usage-guide-as-html-inside-the-release-zip.md
created: 2026-10-24
started: 2026-10-24
plan: none
milestone: M5 Trust
changelog: housekeeping
---

# 122 — Tests of the usage guide generator's error paths

## Context

Unit tests of `scripts/usage-guide.py`'s rejections (a marker out of place, Markdown the page cannot render, a
link to a missing file, a render that is not deterministic) over a fixture README, run where the `--check`
runs in the `lint` job; the repo's first Python test file, in a shape its other scripts can follow.
Why: 120 verified those rejections by hand once; the generator runs in CI and in `release.ps1`, and a silent
change to a rejection would reach the zip.
Done when: each documented rejection has a test that fails without it, the tests run in CI, and CONTRIBUTING's
checks name the command.

Decided 2026-10-24: the standard library's `unittest` only (D-002, nothing to install), the fixture READMEs
made inside the tests or under a folder next to the script, never a copy of the real README.

## Docs that move

- `CONTRIBUTING.md`: the test command in "Checks before a commit"
- `docs/development.md`: "The usage guide" names the tests and how a new rejection gets one; the Layout
  bullet of `scripts/` if the tests live there

## Not in this change

- The generator's behavior (a rejection the tests reveal as wrong is a follow-up, not a fix here)
- Tests of `changelog.py`, `coverage-check.py` or `sync-issues.py` (the shape is set here, the others follow
  later if wanted)

## Steps

1. [ ] The test module (`scripts/tests/test_usage_guide.py` or the name the docs decide) with `unittest`,
       importing the script as a module; one test per documented rejection, each asserting the exit and the
       README line number in the message, and one happy-path test over a small fixture README that renders
       deterministically
2. [ ] The command (`python -I -m unittest discover -s scripts/tests` or the like) in the `lint` job next to
       the `--check` step, in CONTRIBUTING and in the docs
3. [ ] Verification: the tests pass locally and in CI; each test fails when its rejection is disabled (tried
       once by hand, the Log says which)

## Log

- 2026-10-24 — promoted

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
