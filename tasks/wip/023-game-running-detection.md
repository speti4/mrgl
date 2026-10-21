---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M2 Window MVP
---

# 023 — Game-running detection: no `WTF` write while the client runs

## Context

The client rewrites `AddOns.txt` on logout and holds `WTF` files open; a write during a session is lost or
corrupts a file. A new core module finds a running process whose executable lives under the flavor's folder
(by path, not by name: the Forever executable's name is unknown before launch), behind a trait that lists
the executable paths of running processes, with the Windows implementation and a stub for tests. It gives
the two calls the later tasks use: a guard every `WTF` write (a backup restore of 013, an addon set of 014
and 027, a SavedVariables cleanup of 025) calls and that refuses with the documented message while the
client runs, and a check the install engine (004) turns into a warning (the client reads addons at
`/reload`). The voice gets a line for both; `scan` and `list` report the running client in their summary
and in `--json`.
Done when: a test process started from a fixture flavor folder is detected, the guard refuses with the
documented message, and the check costs nothing when no process runs.

Decided 2026-10-21: M2, before `mrgl doctor` (024), so the doctor's report and the install warning are in
the first release; the process crate or the Windows API binding carries its one-line reason in
`Cargo.toml` (D-002) and a license on the allowlist.

## Docs that move

- `crates/mrgl-core/Cargo.toml`: the process crate joins with its reason; `deny.toml` if a license needs
  the allowlist.
- `docs/development.md` ("Layout"): the new module.
- `docs/nerglish.md`: the two lines (the refusal, the warning) if they add a vocabulary entry.

## Not in this change

- The `WTF` writes themselves (013, 014, 025, 027) and the install warning's call site in the engine
  (004 calls the check once both are merged).
- The doctor report (024).

## Steps

1. [ ] The module: the process-list trait, the Windows implementation, the stub, `client_running(flavor)`
2. [ ] The guard for `WTF` writes and the install check, both with their voice lines
3. [ ] Test: a copy of a tiny synthetic executable (or the test's own) started from a fixture flavor folder
       is detected; the no-process path is cheap
4. [ ] `scan` and `list`: the running client in the summary and in `--json`; end-to-end tests with the stub
5. [ ] Docs under *Docs that move*; `cargo deny check`; every check green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, when the work is done)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the stale index is the expected finding on a task branch)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
