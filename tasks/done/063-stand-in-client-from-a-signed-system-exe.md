---
size: S
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
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

1. [x] The two `spawn_stand_in` helpers copy a signed system executable (`ping.exe`, or a better choice
       justified in the Log) to `client-stand-in.exe` in the fixture flavor folder and run it blocking
       (`-n` with a long count against the loopback address); the ignored `stand_in_client` test bodies go
2. [x] The tests kill the child at the end (no stdin close); every existing assertion unchanged
3. [x] `cargo test` twice in a row on this machine with no quarantine event; the Verification says so
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — the executable is `ping.exe` of the system folder (`SystemRoot` + `System32`), run as
  `-n 3600 127.0.0.1` with stdin, stdout and stderr to null: signed by Microsoft (catalog), present on every
  Windows install and every CI runner, it blocks for an hour without a console, a network beyond the
  loopback or any input. A renamed copy runs as the original does (`client-stand-in.exe -n 4 127.0.0.1`
  returned after 3 seconds, exit 0). `timeout.exe` was not taken: it refuses a redirected stdin.
- 2026-10-22 — `spawn_stand_in` returns `Option<StandIn>`: `None` when `SystemRoot` is unset or has no
  `System32\ping.exe`, and the test prints `skipped: no ping.exe in the system folder to start the stand-in
  client` and returns. `StandIn` holds the child: `id()` for the assertions, `stop()` kills and waits it
  (where the tests called `kill` and `wait` before), and its `Drop` kills and waits a child not yet
  stopped, so a failing assertion leaves no process behind. The helper and the guard are `#[cfg(windows)]`
  like every test that uses them. The ignored `stand_in_client` bodies and the `std::io::Read` imports
  they needed are gone; every assertion is unchanged.

## Deviations from the plan

- A third copy of the helper lived in `crates/mrgl-core/tests/backup.rs` (the restore refused while the
  client runs, task 013); it got the same change, or one test binary would still copy itself.
- A non-Windows target has no stand-in test at all, as before: the tests that spawn it were already
  `#[cfg(windows)]` (the macOS process list is task 036), so the skip path is the Windows one without a
  `ping.exe`.

## Verification

- 2026-10-22 — `cargo test --locked` twice in a row in the worktree, both green (exit 0), with the temp
  folder of the test processes pointed at a scratch folder and a watcher hashing every `.exe` written under
  it: each run wrote exactly five, all `client-stand-in.exe` (core `client` detect, core `backup` client,
  CLI scan, CLI restore-client, CLI install client-running), each byte-equal (SHA-256) to the system's
  `ping.exe`, 45 056 bytes; no copy of a test binary appeared. After each run no `client-stand-in` process
  was left and no `.exe` remained under the scratch temp folder. No test reports as ignored any more.
- 2026-10-22 — Windows Defender exclusions for the repo folder and the temp folder were active on the
  machine during these runs, so the local runs cannot show a quarantine either way; `Get-MpThreatDetection`
  lists no detection after the ones that led to this task. The proof of the fix is the design (no copy of
  the test binary is written) and the green CI run.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
