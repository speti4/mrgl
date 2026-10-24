---
size: S
origin: done/117-test-hardening-toward-full-line-and-branch-coverage.md
created: 2026-10-23
started: 2026-10-24
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

1. [ ] Choose the shared place (a dev-only test-support module of `mrgl-core` behind a feature the two other
       crates enable as a dev-dependency, or a workspace-internal test crate), with the one-line reason in
       `Cargo.toml` as D-002 asks
2. [ ] Move the stand-in client and its drop guard there, with the skip rule, and the core's locked file helpers
3. [ ] Every test file (`crates/mrgl-core/tests/{client,backup,sets,savedvars}.rs`, the unit tests of
       `crates/mrgl-core/src/discover.rs`, `crates/mrgl-cli/tests/cli.rs`, `crates/mrgl-app/tests/toolbox.rs`)
       uses the one definition; the copies go
4. [ ] `docs/git-workflow.md` lists `coverage` among the required checks
5. [ ] `cargo test --locked` green on every crate; the coverage run stays at 100% lines and branches

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
