---
size: S
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 063 — The stand-in client of the tests is a copy of a signed system executable, not of the test binary

## Context

The client-running tests (023, `crates/mrgl-core/tests/client.rs` and `crates/mrgl-cli/tests/cli.rs`)
copy the test binary itself into the fixture flavor folder as `client-stand-in.exe` and spawn the copy; the
copy becomes one of a signed system executable that blocks on its own (`ping.exe` with a long count, or a
better one), killed by the test at the end, with the same assertions. The pattern, not the code, is what an
antivirus heuristic reads as a dropper: a binary that copies itself and runs the copy. Windows Defender
quarantined the test binaries and their stand-in copies on a developer machine as a machine-learning
detection, which fails and relinks `cargo test` at random.
Done when: the stand-in tests pass with no copy of the test binary written anywhere, and `cargo test`
runs clean on a machine with Windows Defender's default settings.

Decided 2026-10-22: the fix is the test's, not the client module's (`client.rs` stays as it is); the
stand-in keeps its name `client-stand-in.exe` so every assertion on the name holds; on a target where no
signed blocking executable is known the test is skipped with a message, never a self-copy.

## Docs that move

- `docs/development.md` ("Fixtures", or wherever the stand-in client is described): how the stand-in is
  made and why not from the test binary.

## Not in this change

- Signing the release executables (M5); any change to the detection itself (023).

## Steps

1. [ ] The two `spawn_stand_in` helpers copy a signed system executable (`ping.exe`, or a better choice
       justified in the Log) to `client-stand-in.exe` in the fixture flavor folder and run it blocking
       (`-n` with a long count against the loopback address); the ignored `stand_in_client` test bodies go
2. [ ] The tests kill the child at the end (no stdin close); every existing assertion unchanged
3. [ ] `cargo test` twice in a row on this machine with no quarantine event; the Verification says so
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, at close)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
