---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M2 Window MVP
closed: 2026-10-21
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

Decided 2026-10-21: the process list comes from `windows-sys` (`CreateToolhelp32Snapshot`, `OpenProcess`
with the limited query right, `QueryFullProcessImageNameW`), not from a process crate: the crate was
already in `Cargo.lock` through Tauri, so no new crate joins the tree, its MSRV (1.71) is under 1.90 and its
license (MIT OR Apache-2.0) is on the allowlist; `sysinfo` would have added a tree of crates for two calls.
The other targets list nothing until the macOS preview (M6).

Decided 2026-10-21: the refusal is `Error::ClientRunning`, whose plain line is `the client is running in
<folder> (<exe>, pid <n>): close the game first, nothing under WTF is written while it runs`; the install
warning is `the client is running in <folder> (<exe>, pid <n>): the installed addons load at /reload or
the next login`. Both use the existing error and warning kinds of the voice table: no new vocabulary.

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

1. [x] The module: the process-list trait, the Windows implementation, the stub, `client_running(flavor)`
2. [x] The guard for `WTF` writes and the install check, both with their voice lines
3. [x] Test: a copy of a tiny synthetic executable (or the test's own) started from a fixture flavor folder
       is detected; the no-process path is cheap
4. [x] `scan` and `list`: the running client in the summary and in `--json`; end-to-end tests with the stub
5. [x] Docs under *Docs that move*; `cargo deny check`; every check green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `crates/mrgl-core/src/client.rs`: `ProcessList` (`SystemProcesses` on windows-sys,
  `FixedProcesses` as the stub), `RunningClient {pid, exe}`, `client_running`, `client_running_among`,
  `ensure_client_not_running`, `warn_if_client_running`; `error.rs` gains `ClientRunning` and
  `ProcessList`; the flavor folder is canonicalized once and the `\\?\` prefix stripped, the process paths
  are compared as the kernel reports them, component by component and case-insensitively, so a junction
  into the game folder and a differently cased path still match. Five unit tests over the stub.
- 2026-10-21 — `tests/client.rs`: the test binary copied into a synthetic flavor folder under the temp dir
  and started as a stand-in client (an ignored test that waits for its stdin to close) is detected with
  its pid, the guard refuses with the documented line, the warning is the documented line, and after a
  kill the check is negative; ten checks over the fixture tree run in well under the 5 s budget (0.26 s
  for the whole test binary). The CLI tests run the same stand-in end to end: `scan` and `list` add
  `client running` to the summary line and `client_running: {pid, exe}` to the JSON, null without one;
  an error listing the processes is a warning line, never a failed scan.
- 2026-10-21 — `windows-sys` joins `[workspace.dependencies]` with its reason and the three features, under
  `[target.'cfg(windows)'.dependencies]` of `mrgl-core`; `Cargo.lock` changes by one line (no new crate).
  `deny.toml` needs no change. `docs/nerglish.md` needs no change: the two lines use the error and the
  warning kind.
- 2026-10-21 — CI's first run failed the CLI test: the runner's temp folder is an 8.3 short path
  (`RUNNER~1`), and Windows reports an image path as the process was started, short components kept,
  while the flavor folder was canonicalized to its long form. Reproduced locally by starting a process
  through a short path. Fixed in core, since a client launched through such a path is the same case: the
  flavor folder is resolved into both forms (`GetShortPathNameW`, one call, still no per-process I/O) and
  each component of a process path matches either form; two unit tests cover mixed forms and a process
  started through the short name. The unit tests' made-up paths moved to a drive letter that exists
  nowhere, so no test resolves against a real folder.

## Deviations from the plan

- The CLI tests use the real process list with the stand-in client rather than the stub: the stub would
  have needed a test hook in the shipped exe (an environment variable naming a process list), and the real
  round trip costs nothing. The stub stays in core for the engine's and the toolbox's tests.
- The engine's check is `warn_if_client_running(flavor) -> Option<String>`: a failed process list becomes
  the warning line rather than an error, so an install never fails on it; the guard, the other way round,
  refuses on a failed process list rather than risk the write.

## Verification

- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (35 core unit tests, 2 client and 10 scan integration tests, 24 CLI tests, the app's tests),
  `cargo +1.90 check --all-targets --locked`, `cargo deny check` (advisories, bans, licenses, sources ok),
  `python -I scripts/changelog.py --check`, all green locally on Windows 11.
- 2026-10-21 — the real-process test: the stand-in started from the temp flavor folder is reported with
  its pid and `client-stand-in.exe` under the flavor folder; `ensure_client_not_running` returns the
  documented refusal; after `kill` and `wait` the check is `None` and the guard passes. The same tests
  pass with `TEMP` set to its 8.3 short form, the CI runner's case.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change (`deny.toml` and `docs/nerglish.md` were
      conditional and needed nothing, see the Log)
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
