---
size: S
origin: done/004-install-engine-with-state-sidecar.md
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
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

1. [x] The engine's commit calls the running-client check before the first move; the result carries the
       warning text (the `install_warning` line of 023)
2. [x] `add <file.zip>` and `remove` report the warning through the voice and in `--json`
3. [x] Tests: the stand-in client running (the warning, the install still done) and not running (nothing)
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `install.rs`: `Engine` holds the process list behind the `ProcessList` trait of 023 (a
  private field, the system's list from `Engine::new` and `Engine::with_home`; `Engine::with_processes`
  hands it another, a `FixedProcesses` stub in tests). `commit` asks `client::client_running_among` once,
  after the stale folders are known and before the first move, and `remove` once after the entry is found
  and before the first folder goes; the line lands in the new field `client_warning: Option<String>` of
  `Installed` and `Removed`. A process list that cannot be taken is a warning too (its error line), as
  `warn_if_client_running` does; the write goes on either way.
- 2026-10-21 — `client.rs`: `RunningClient::remove_warning`, the removal's own line ("the removed addons
  stay loaded until /reload or the next login"); the module doc names the engine's two lines.
- 2026-10-21 — `mrgl add <file.zip>` and `mrgl remove` print the client line first among the warnings,
  through the voice's warning kind, and carry it as `client_warning` (a string or `null`) in their JSON
  document, apart from `warnings`. `add --dry-run` writes nothing and does not ask.

## Deviations from the plan

- The removal has its own warning line (`remove_warning`) instead of the install line, whose "the
  installed addons load" would be wrong after a removal.
- The client line is a field of its own on the results and in the JSON (`client_warning`), not one more
  entry of `warnings`: a caller (the window's toast, `add` from a source in 040) can tell it apart, and the
  existing `warnings` stay as they were.

## Verification

- 2026-10-21 — `cargo test --locked`: every test green. `tests/install.rs`: with a `FixedProcesses` stub
  whose second process is `client-stand-in.exe` (pid 4242) in the sandbox's `_forever_` folder, the
  install of `one-folder-1.0.0.zip` reports `the client is running in _forever_ (client-stand-in.exe, pid
  4242): the installed addons load at /reload or the next login`, the folder and the sidecar entry are
  written, the update to 1.1.0 warns and writes `new.lua`, and the removal reports the remove line and
  empties `Interface\AddOns`. With a `_retail_` client and one in a sibling `_forever_2` folder, and with
  the default system list, nothing is reported. A process list that fails reports `cannot list the
  running processes: no snapshot` and the install and the removal still happen.
- 2026-10-21 — `crates/mrgl-cli/tests/cli.rs` (Windows): with the stand-in client of 023 really running
  from the temporary `_forever_` folder (a copy of the test binary, never a game file), `mrgl add
  one-folder-1.0.0.zip --flavor forever` prints three two-line messages (progress, the warning with the
  stand-in's pid, `installed OneFolder 1.0.0 …`) and installs; `--json add` of 1.1.0 carries the line in
  `client_warning` with `warnings` empty; `--serious remove OneFolder` prints the remove line and then
  `removed OneFolder 1.1.0 (1 folder: OneFolder) from _forever_` and the folder is gone. Once the stand-in
  is killed, `--json add` and `--json remove` carry `client_warning: null`.
- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90
  check --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-21`
