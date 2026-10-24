---
size: S
origin: done/117-test-hardening-toward-full-line-and-branch-coverage.md
created: 2026-10-23
started: 2026-10-24
closed: 2026-10-24
plan: none
milestone: M5 Trust
changelog: housekeeping
---

# 118 — One stand-in client helper for the tests of the three crates

## Context

The stand-in game client the tests spawn (`ping.exe` copied into a temp flavor folder as the client's
executable, with its drop guard) is written once, in a place the tests of the core, the command line and the
window share, instead of the copy each test file carries today (seven files); the same place takes the locked
file helpers of the core's tests.
Why: 117 added cases to every copy; a change to the stand-in (a new flag, a new skip rule) is seven edits.
Done when: one definition, every test file uses it, and `cargo test --locked` is green on every crate.

Also here: `docs/git-workflow.md` names `coverage` among the checks the ruleset on `main` requires (it has
been required since 2026-10-23, `docs/development.md` says so already).

## Docs that move

- `docs/development.md`: the layout and the Fixtures or Tests paragraph, where the shared helper lives and how
  a test reaches it; the Coverage section if the helper's exclusion needs a line
- `docs/git-workflow.md`: `coverage` in the list of required checks
- `CONTRIBUTING.md`: the test conventions, if they name the stand-in

## Not in this change

- A new flag or skip rule of the stand-in (the helper is moved as it is)
- The macOS stand-in (036)

## Steps

1. [x] Choose the shared place (a dev-only test-support module of `mrgl-core` behind a feature the two other
       crates enable as a dev-dependency, or a workspace-internal test crate), with the one-line reason in
       `Cargo.toml` as D-002 asks
2. [x] Move the stand-in client and its drop guard there, with the skip rule, and the core's locked file helpers
3. [x] Every test file (`crates/mrgl-core/tests/{client,backup,sets,savedvars}.rs`, the unit tests of
       `crates/mrgl-core/src/discover.rs`, `crates/mrgl-cli/tests/cli.rs`, `crates/mrgl-app/tests/toolbox.rs`)
       uses the one definition; the copies go
4. [x] `docs/git-workflow.md` lists `coverage` among the required checks
5. [x] `cargo test --locked` green on every crate; the coverage run stays at 100% lines and branches

## Log

- 2026-10-24 — promoted
- 2026-10-24 — The shared place is a workspace crate, `crates/mrgl-test-support` (`publish = false`, the
  standard library only), a dev-dependency of `mrgl-core`, `mrgl-cli` and `mrgl-app` through
  `[workspace.dependencies]`, each line with its reason. Chosen over a `test-support` feature of `mrgl-core`:
  the core's own integration tests would need the crate as a dev-dependency of itself to turn the feature on,
  and the shipped library would carry test code behind a feature; a crate of its own keeps the exes' graphs
  free of it (`cargo tree -e normal,build -p mrgl-app -p mrgl-cli` does not name it), needs no new license or
  source in `cargo deny`, and the license page skips it as a private workspace crate. Its two modules carry
  the documented `coverage(off)` attribute, listed in `docs/development.md`, "Coverage", as test code like a
  `#[cfg(test)]` module.
- 2026-10-24 — `mrgl_test_support::stand_in`: `spawn_stand_in`, the drop guard `StandIn` (`id`, `stop`) and
  the skip line as the constant `SKIPPED`, moved as they were (the six copies were the same code; one doc
  comment differed in wording). `mrgl_test_support::locked`: `file` (no sharing), `file_sharing_reads` and
  `folder` (no sharing, the backup semantics flag), the three shapes of handle the tests opened inline.
- 2026-10-24 — The six copies are gone: `crates/mrgl-core/tests/{client,backup,sets,savedvars}.rs`,
  `crates/mrgl-cli/tests/cli.rs` and `crates/mrgl-app/tests/toolbox.rs` import the stand-in and print
  `SKIPPED` where `ping.exe` is missing. The held handles of `crates/mrgl-core/tests/{backup,install,scan,sets}.rs`,
  of the unit tests in `crates/mrgl-core/src/staging.rs` and `crates/mrgl-core/src/savedvars.rs` and the one
  in `crates/mrgl-cli/tests/cli.rs` call `locked`. The module comment of `crates/mrgl-cli/tests/cli.rs` said
  the running-client tests copy the test binary (the state before task 063); it names the stand-in now.

## Deviations from the plan

- Step 3 lists the unit tests of `crates/mrgl-core/src/discover.rs`; they carry no copy of the stand-in.
  Their `TestKey` writes a registry key through the system's `reg.exe` (a spawned system exe like the
  stand-in, which its comment compares it to), a helper of that one file, so it stays where it is; the core's
  unit tests reach `mrgl-test-support` all the same (`staging.rs` and `savedvars.rs` use `locked`).
- The core's tests had no locked file helper functions to move: every test opened its handle inline. The
  three shapes they used became the functions of `locked`. Three handles of other shapes stay inline, one use
  each: the junction handle of `crates/mrgl-core/tests/link.rs` (the reparse point flag), the handle of a
  `crates/mrgl-core/src/doctor.rs` unit test that shares reads and writes, and the handle a
  `crates/mrgl-core/src/backup.rs` unit test opens inside a stub rename that returns an `io::Result`.
- The skip line is a constant of the crate (`SKIPPED`) that the tests print; the rule and the words are the
  ones of the copies.
- `CONTRIBUTING.md` did not move: its test conventions name neither the stand-in nor a shared helper; they
  link `docs/development.md`, whose Layout and Fixtures paragraphs now say where the helpers live.
- `docs/git-workflow.md`: the bullet with the list of required checks is rewrapped to the line width.

## Verification

- 2026-10-24 — `cargo fmt --check` clean; `cargo clippy --all-targets --locked -- -D warnings` clean;
  `cargo test --locked` green (30 test binaries, 0 failed); `cargo test --locked -p mrgl-core --test client
  -- --nocapture` ran the stand-in test without the skip line; `cargo +1.90 check --all-targets --locked`
  clean; `cargo deny check`: advisories, bans, licenses and sources ok; `python -I scripts/changelog.py
  --check` green after the regeneration.
- 2026-10-24 — `grep -rn "fn spawn_stand_in\|struct StandIn" crates` names `crates/mrgl-test-support` only;
  `cargo tree --locked -e normal,build -p mrgl-app -p mrgl-cli` does not name `mrgl-test-support`.
- 2026-10-24 — Coverage, `cargo +nightly-2026-10-21 llvm-cov --workspace --branch --locked --lcov`
  (cargo-llvm-cov 0.9.1) and `python -I scripts/coverage-check.py`: 100.00% lines (0 missed of 15577),
  100.00% branches (0 missed of 2276); mrgl-core 11321 / 1756, mrgl-cli 2516 / 392, mrgl-app 1740 / 128, the
  numbers of task 117's close; 86 lines and 22 branches marked `coverage: os`; 5 reasoned `coverage(off)`
  sites in the code, 5 in the docs list; `mrgl-test-support` is not in the counts. From the worktree's own
  path the window crate was not measured at all: its rustc command line there is 37,656 characters, over the
  32,767 Windows allows, so cargo hands rustc an argument file and cargo-llvm-cov's wrapper, which looks for
  `--crate-name`, does not instrument the crate. The run above was made from a `subst` drive at the
  worktree's root; there cargo-llvm-cov's default pattern for test files does not match (the root ends in a
  backslash), so the lcov records under `tests/` were dropped before the check.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: YYYY-MM-DD`
