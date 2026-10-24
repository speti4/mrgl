---
size: S
origin: done/118-one-stand-in-client-helper-for-the-tests-of-the-three-crates.md
created: 2026-10-24
started: 2026-10-24
closed: 2026-10-24
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

1. [x] Reproduce once from a long path and once from a `subst` drive, and note what each run reports
2. [x] `CONTRIBUTING.md`: the paragraph (the short path, the symptom and its cause in one or two sentences, the
       `subst` commands and the mapping's removal; a drive letter a test assumes absent, such as `Q:`, is not used)
3. [x] `docs/development.md`: the Coverage section points at the paragraph or repeats the rule, whichever it
       does today for the local run
4. [x] Verification: the `subst` run reaches the numbers of the `coverage` job on the same tip

## Log

- 2026-10-24 — promoted
- 2026-10-24 — The cause measured: the nightly cargo of the coverage pin passes one
  `-L dependency=<root>\target\llvm-cov-target\debug\build\<crate>\out` per dependency, 187 for a window test,
  so the window crate's instrumented rustc line holds the checkout root 204 times: 41,076 characters at a
  109-character root, 30,672 at 58, a limit of 68 characters for the root at this tip (the plain build of the
  stable toolchain stays near 3,000 characters). Past the limit cargo retries with an argument file and the
  wrapper of cargo-llvm-cov 0.9.1 no longer sees `--crate-name`.
- 2026-10-24 — The `tests/` records of a `subst` run: cargo-llvm-cov 0.9.1 builds its default pattern for test
  files as `^<workspace root>(\\.*)?\\(tests|examples|benches)\\` (`\\` a backslash); with a drive root
  (`W:\`) as the workspace root, which ends in a backslash, the pattern needs a doubled separator and never
  matches. Decided: the drive maps the folder that holds the checkout and the run starts in the checkout's
  folder on it, so the block of CONTRIBUTING.md runs as written, with no extra flag and no change to
  `scripts/coverage-check.py` (`--ignore-filename-regex` would have meant a flag the `coverage` job does not
  pass).
- 2026-10-24 — `CONTRIBUTING.md`: the paragraph and the `subst` block after the coverage commands; the block's
  comment named the summary as JSON, now the export as lcov. `docs/development.md`, "Coverage": one sentence
  pointing at the paragraph, as the section already points at CONTRIBUTING.md for the local commands.

## Deviations from the plan

- Step 1 took four runs instead of two: the worktree's own root (58 characters) is under the limit and measured
  everything, so the symptom was reproduced from a junction at a deeper path (109 characters). A junction at 170
  characters, tried first, failed differently: `link.exe` exits with LNK1104 and the run stops with an error.
- The `subst` drive maps the checkout's parent folder, not the checkout: the plan named the `subst` way, the
  measurement showed that a drive root breaks cargo-llvm-cov's pattern for test files.
- The coverage block's comment in `CONTRIBUTING.md` ("the summary as JSON") was corrected to the lcov export
  task 117 moved to.

## Verification

- 2026-10-24 — CONTRIBUTING.md's coverage block (`cargo +nightly-2026-10-21 llvm-cov --workspace --branch
  --locked --lcov`, cargo-llvm-cov 0.9.1, then `python -I scripts/coverage-check.py`) on f68d451, the code of
  main a01df4a:
  - from the worktree's own path (58-character root): green, 100.00% lines (0 missed of 15577), 100.00%
    branches (0 missed of 2276), mrgl-app 1740 / 128, mrgl-cli 2516 / 392, mrgl-core 11321 / 1756;
  - from a junction at a 109-character root: red, 99.97% lines (4 missed of 13837), 99.91% branches (2 missed
    of 2148); no `mrgl-app` row, `mrgl-core/src/update.rs` 4 lines and 2 branches missed; no error from cargo;
  - from a junction at a 170-character root: cargo exits 101 on LNK1104, and the check, which then read the
    export of the earlier run, printed that run's numbers;
  - from `subst W:` mapped at the checkout's parent, the run in `W:\119-short-coverage-path`: green, 100.00%
    lines (0 missed of 15577), 100.00% branches (0 missed of 2276), the three crates with the numbers above, 37
    source files in the export and none under `tests/`; `subst W: /d` afterwards, `subst` lists no mapping.
  - The `coverage` job on main a01df4a (run 40105059442): 15577 lines and 2276 branches, the same three rows.
- 2026-10-24 — `cargo fmt --check` clean; `cargo clippy --all-targets --locked -- -D warnings` clean; `cargo test
  --locked` green; the gate over the `subst` run's export with the edited docs: 5 reasoned sites in the code, 5 in
  the docs list.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: YYYY-MM-DD`
