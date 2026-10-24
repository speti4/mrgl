---
size: S
origin: done/118-one-stand-in-client-helper-for-the-tests-of-the-three-crates.md
created: 2026-10-24
started: 2026-10-24
plan: none
milestone: M5 Trust
changelog: housekeeping
---

# 119 — The local coverage run needs a short checkout path

## Context

CONTRIBUTING's coverage paragraph says that the local measurement runs from a short checkout path (a `subst`
drive or a short folder), and names the symptom of a long one: the window crate's rustc command line exceeds
the 32,767 characters Windows allows, cargo hands rustc an argument file, cargo-llvm-cov's wrapper then
misses `--crate-name` and does not instrument the crate, and the run reports `mrgl-app` missing and core
lines missed.
Why: a checkout under a long folder (a git worktree with a long branch name) measures a wrong result without
an error; CI is not affected, so the local run is the only place it bites.
Done when: the paragraph is in CONTRIBUTING.md and a run from a `subst` drive of the repo reaches the same
numbers as the `coverage` job.

## Docs that move

- `CONTRIBUTING.md`: the coverage paragraph of "Checks before a commit" (the short path, the symptom, the
  `subst` way and its removal afterwards)
- `docs/development.md`: the Coverage section, if it repeats the local-run instructions or should point at
  CONTRIBUTING for them

## Not in this change

- A change to `scripts/coverage-check.py` or to the CI job (CI is not affected)
- A shorter worktree naming scheme for the maintainer sessions (`docs/maintainer-sessions.md` stays)

## Steps

1. [ ] Reproduce once from a long path and once from a `subst` drive, and note what each run reports
2. [ ] `CONTRIBUTING.md`: the paragraph (the short path, the symptom and its cause in one or two sentences, the
       `subst` commands and the mapping's removal; a drive letter a test assumes absent, such as `Q:`, is not used)
3. [ ] `docs/development.md`: the Coverage section points at the paragraph or repeats the rule, whichever it
       does today for the local run
4. [ ] Verification: the `subst` run reaches the numbers of the `coverage` job on the same tip

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
