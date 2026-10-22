---
size: L
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-23
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
3. [x] The command line's missing end-to-end cases and the unit tests of its pure helpers (pull request 3)
4. [x] The window's commands under Tauri's mock runtime, generic over the runtime, with the busy and
       refused arms; the floors at 100; the close (pull request 4)
5. [x] Verification: the job summary's table at 100 / 100 outside the listed exclusions, the register check
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
- 2026-10-23 — Pull request 3: mrgl-cli at 100.00% lines and branches (2516 lines, 392 branches; 4 lines
  marked `coverage: os`). The key command, the confirmations of `backup` and `update` take their input and the
  terminal flag as parameters; `key set` and `key clear` run end to end against the folder key store; a cancelled
  update confirmation now ends in the summary's `not updated (cancelled)` count the core already worded; the
  quiet exit on a closed pipe writes the profile under `cfg(coverage)`, since `std::process::exit` skips the
  handler that writes it. The review of pull request 2 moved the folder key store behind `folder:<path>`.
- 2026-10-23 — Pull request 4: mrgl-app at 100.00% lines and branches (1740 lines, 128 branches). The
  commands that take a Tauri handle are generic over the runtime, so the command tests build them on
  `tauri::test`'s mock runtime (the `test` feature as a dev-dependency only); the commands that read the
  process environment run in a child of the test binary with the environment set on it, as the command line's
  tests run the exe; the window's build script links the resource library into the test binaries, whose
  manifest the menu crate's import needs to start at all. The floors stand at 100 / 100; `coverage` joins the
  required checks of the ruleset. Follow-up filed: 118 (one stand-in client helper for the three crates).
- 2026-10-23 — Closed with pull request 4.

## Deviations from the plan

- The gate reads the lcov export, not the JSON summary (the summary counts every instantiation of a generic
  function on its own and disagrees with the report a reader acts on), and a second exclusion form joined the
  attribute: the `// coverage: os` line mark for a failure arm only the operating system can provoke, checked
  against the measurement. The CLI's e2e tests reach core lines no core test does, so the measure is the
  workspace run.
- A cancelled `update` confirmation ends in the summary's `not updated (cancelled)` count instead of a return
  before the summary (the core's wording, the window's behaviour); the folder key store is `folder:<path>`
  after the review of pull request 2.

## Verification

- 2026-10-23 — `cargo +nightly-2026-10-21 llvm-cov --workspace --branch --locked --lcov` (cargo-llvm-cov 0.9.1)
  then `python -I scripts/coverage-check.py`: 100.00% lines (0 missed of 15577), 100.00% branches (0 missed of
  2276); mrgl-core 11321 / 1756, mrgl-cli 2516 / 392, mrgl-app 1740 / 128; 86 lines and 22 branches marked
  `coverage: os`; 4 reasoned `coverage(off)` sites in the code, 4 in the docs list, no stale mark. The
  `coverage` job of pull requests 1 to 3 measured the same numbers as the local runs (2026-10-22: 86.35% /
  81.57%; pull request 2: 94.93% / 95.67%; pull request 3: 96.58% / 98.25%).
- 2026-10-23 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`
  (27 targets green), `cargo +1.90 check --all-targets --locked`, `cargo deny check`, `uvx zizmor .github`,
  `python -I scripts/changelog.py --check`; `cargo tree -e normal,build,features -p mrgl-app` names no
  `tauri` feature `test`, so the exe never carries the mock runtime; `cargo build --release -p mrgl-app` builds.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: 2026-10-23`
