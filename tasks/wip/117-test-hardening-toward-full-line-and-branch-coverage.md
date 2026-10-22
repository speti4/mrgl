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

1. [x] The exclusion attribute (`cfg_attr(coverage_nightly, coverage(off))` with a `// coverage: off —`
       reason line), the `unexpected_cfgs` lint, the two pin files, `scripts/coverage-check.py` with the
       floors and the exclusion register, the `coverage` CI job, the docs (pull request 1)
2. [x] The core's seams (a process list for the client guards, the key store from the environment with a
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
- 2026-10-22 — Pull request 1: the exclusion attribute with its reason line and the `unexpected_cfgs`
  declaration, the two pins, `scripts/coverage-check.py` (the table, the floors, the register against the
  docs list), the `coverage` job, the docs. With the test modules excluded the baseline is 86.33% lines
  (2204 missed of 16122) and 81.53% branches (437 of 2366); the floors start at 86.0 / 81.0. Four reasoned
  exclusions: the window's `main` and `run`, `choose_root`, the Credential Manager module.
- 2026-10-22 — Pull request 1 merged (#277) after two fixes on its branch: the summary folder, and the gate
  reads the lcov export instead of the JSON summary (the summary counts every instantiation of a generic
  function on its own, so a branch one instantiation never takes shows as missed while the report shows it
  covered); the `// coverage: os` line mark for a failure arm only the operating system can provoke, checked
  against the measurement (a marked line a test reaches fails the run).
- 2026-10-23 — Pull request 2: mrgl-core at 100.00% lines and branches (11313 lines, 1754 branches counted;
  79 lines and 22 branches marked `coverage: os`). Seams: `ensure_client_not_running_among` and
  `warn_if_client_running_among`, `keys::store_from_env` with the folder store behind `MRGL_KEY_STORE=<folder>`,
  `backup::move_tree_with` and the `_at` forms taking the clock, `restore_wtf_with`, `discover::resolve_root_from`
  and an environment parameter for the usual install folders, `sources::http_from_env_value`,
  `staging::RunDir::create_at`, `install::move_dir_with` and `swap_or_revert`. The registry reads are tested
  on well-known keys of the real registry; the dead arms found on the way are gone with a comment each;
  `zip_entry::Refusal` carries the bound's refusal so the restore needs no catch-all arm; the unused
  `keys::list_of_process` is removed. Workspace: 94.93% lines, 95.67% branches; floors 94.5 / 95.5.

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
