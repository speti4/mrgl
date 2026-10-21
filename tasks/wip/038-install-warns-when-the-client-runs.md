---
size: S
origin: done/004-install-engine-with-state-sidecar.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M2 Window MVP
---

# 038 — The install engine warns when the client runs

## Context

The install engine's commit (004) calls the running-client check of 023 at the place marked for it
before the first move, and turns a running client into a warning in the install result and the voice's
warning line (the client reads addons at `/reload`); never a refusal, unlike the `WTF` writes. Both
halves exist on `main`; the call site was left out so the two branches could merge apart.
Done when: an install with the stand-in client of 023 running reports the warning and still installs,
and without it reports nothing.

Decided 2026-10-21: the check runs once per commit, not per folder, and the engine takes the process
list behind the trait of 023 so the test injects the stand-in client without a real process.

## Docs that move

- `docs/development.md` ("Layout"): the engine's commit names the running-client check.
- `crates/mrgl-core/src/install.rs` module doc: the warning, and that it is never a refusal.

## Not in this change

- The `WTF` refusals (013, 023, unchanged); the window's warning toast (010); `update --all` (006),
  which inherits the warning through the same install call.

## Steps

1. [ ] The engine's commit calls the running-client check before the first move; the result carries the
       warning text (the `install_warning` line of 023)
2. [ ] `add <file.zip>` and `remove` report the warning through the voice and in `--json`
3. [ ] Tests: the stand-in client running (the warning, the install still done) and not running (nothing)
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
