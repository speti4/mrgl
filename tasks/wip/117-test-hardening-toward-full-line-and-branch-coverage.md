---
size: L
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M5 Trust
---

# 117 — Test hardening toward full line and branch coverage

## Context

The tests reach 100% line and branch coverage of the three crates as `cargo +nightly llvm-cov --workspace
--branch` measures it, every uncovered line either tested or excluded with a reason in the code (the Tauri
start-up of `mrgl-app`'s `main.rs` and `lib.rs`, the real Windows Credential Manager behind the fake store,
and the like, listed in `docs/development.md`), the measurement documented there with the tool set-up, and
the exclusion list the only thing below 100%. Measured 2026-10-22: 89.6% lines, 79.6% branches overall
(core 92.1% / 79.6%, cli 88.7% / 84.4%, app 68.2% / 67.6%; `commands.rs` 26.9% lines, `keys.rs` 59.1%
branches, `discover.rs` 64.3%, `backup.rs` 69.4%).
Why: the untested branches are where a real folder meets a case no fixture has; M5 is the hardening milestone.
Done when: the report shows 100% outside the documented exclusions, and a CI job prints the numbers on every
pull request so a drop is seen before the merge.

Decided 2026-10-22: the CI job fails below the floors, which start at the measured numbers and rise with
every pull request that lands tests, to 100 at the close; whether the job joins the required checks is
decided at the close. The task lands over several pull requests and stays in progress between them. The
Windows registry reads are tested on well-known read-only keys of the real registry; the Credential
Manager calls are excluded with their reason.

## Docs that move

- `docs/development.md`: a Coverage section (what is measured, the commands, the two pins and their bump,
  the floors, the exclusion grammar and list, the phantom-branch policy), the job list of "CI and releases",
  the Layout bullets for `scripts/`
- `CONTRIBUTING.md`: the local coverage run in "Checks before a commit"
- `.github/workflows/ci.yml`: the `coverage` job and the header comment

## Not in this change

- A shared test-support crate for the stand-in-client helper the test files copy (a follow-up if the
  core's shared `tests/common` module proves the need)
- The `cfg(not(windows))` twins: compiled by the `non-windows` job, not measured
- The window follow-ups 114, 115 and 116

## Steps

1. [ ] The exclusion attribute (`cfg_attr(coverage_nightly, coverage(off))` with a `// coverage: off —`
       reason line), the `unexpected_cfgs` lint, the two pin files, `scripts/coverage-check.py` with the
       floors and the exclusion register, the `coverage` CI job, the docs (pull request 1)
2. [ ] The core's seams (a process list for the client guards, the key store from the environment with a
       folder store, the rename of the backup move, the clock of the backup names, the registry reads on
       well-known keys) and the core's missing cases, file by file (pull request 2)
3. [ ] The command line's missing end-to-end cases and the unit tests of its pure helpers (pull request 3)
4. [ ] The window's commands under Tauri's mock runtime, generic over the runtime, with the busy and
       refused arms; the floors at 100; the close (pull request 4)
5. [ ] Verification: the job summary's table at 100 / 100 outside the listed exclusions, the register check
       green, the release build without the `test` feature

## Log

- 2026-10-22 — Started on the branch of pull request 1. Measured with nightly 1.101.0 (2026-10-21) and
  cargo-llvm-cov 0.9.1: 89.60% lines (2338 missed of 22470), 79.64% branches (511 of 2510).

## Deviations from the plan

- none yet

## Verification

- (filled at the close)

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
