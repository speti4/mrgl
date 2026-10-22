---
size: S
origin: done/075-cli-output-survives-a-closed-pipe.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 084 — Stale staging folders are swept, so a run that ends mid-install leaves nothing behind

## Context

The install engine removes the download and staging folders of earlier runs under `<flavor-id>\staging\`
before it stages (a folder no running mrgl holds, by age or by a lock file), and the command line lets the
addon in flight finish before it exits on a closed pipe (075) so the sweep has less to do.
Why: a closed pipe at an install's `downloading` or `installing` line ends the process before the
`RemoveOnDrop` guard runs, and the download folder stays under `staging\` forever.
Done when: a planted stale staging folder is gone after the next install of the fixture flavor, the folder
of the run itself is untouched while it runs, and the closed-pipe test of 075 leaves no staging folder.

Decided 2026-10-22: a staging folder is a leftover only by both rules, a lock and an age. Every run folder
holds an exclusive lock (`File::try_lock`) on `.mrgl-run.lock` inside it for as long as the run holds the
folder; the operating system releases the lock when the process ends, however it ends, so a held lock is a
running mrgl, the run's own or another one on the same flavor (the window next to the command line), and
that folder is never touched. A folder whose lock is free or missing is swept only once it is older than an
hour, by the creation time in its name (`<stem>-<nanoseconds since 1970>-<attempt>`), else by its modified
time: the age covers the moment between creating a folder and locking it, and a run of an mrgl from before
this task, which takes no lock. A lock alone would race with that moment; an age alone would sweep a live
run that takes longer than the age. A folder that holds `previous\` is never swept: it holds the folders an
interrupted or failed swap moved aside, which may be the only copy of the version installed before.

## Docs that move

- `docs/development.md`: the staging folder's lifetime (the sweep's rule, what counts as stale) and the
  closed-pipe rule of 075
- `CLAUDE.md`: the `staging\` line under "Data outside the repo", only if the layout changes (a lock file)

## Not in this change

- The zip cache under `cache\` and its housekeeping (072); a sweep of another flavor's staging folder; a
  sweep from the window

## Steps

1. [x] `crates/mrgl-core/src/staging.rs` (new) and `install.rs`: every run folder under
       `<flavor-id>\staging\` is a locked `RunDir`; `Engine::stage` sweeps the leftovers first by the rule
       above; the run's own folders are never touched; a failed removal is a warning on the result
       (`Staged::sweep_warnings`, carried into `Installed::warnings` by the commit), never an error
2. [x] `crates/mrgl-cli/src/main.rs`: on a closed pipe during an install the addon in flight finishes
       (the swap and the sidecar write), then the process exits quietly with 0 as 075 does
3. [x] Tests: a planted stale folder is gone after the next fixture install, the run's own folder stays while
       it runs, and an install into a closed pipe leaves no staging folder (`staging.rs` unit tests,
       `tests/install.rs`, `tests/cli.rs`)
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — the rule decided (see Context): a held lock keeps a folder, a free or missing lock and an age
  of an hour or more sweep it, a `previous\` folder keeps it.
- 2026-10-22 — `crates/mrgl-core/src/staging.rs`: `RunDir::create` makes the fresh run folder (the naming
  of the former `fresh_staging_dir`) and takes the lock on `.mrgl-run.lock` (a lock that cannot be taken
  leaves the folder to the age rule); its drop releases the lock and removes the folder unless `keep` was
  called. `sweep` / `sweep_at` remove the leftovers and return one warning per folder that could not be
  removed: `a leftover staging folder could not be removed: <path>: <reason>`. `LOCK_FILE`, `PREVIOUS_DIR`
  and `STALE_AFTER` (one hour) are public. Seven unit tests.
- 2026-10-22 — `crates/mrgl-core/src/install.rs`: `Staged` holds its `RunDir` (a failed revert calls
  `keep`, as before), `Engine::stage` sweeps before it creates the folder, the commit starts its warnings
  with the sweep's; `install_from_source` downloads into a `RunDir`; `fresh_staging_dir` and
  `RemoveOnDrop` are gone.
- 2026-10-22 — `crates/mrgl-cli/src/main.rs`: an `InFlight` guard held by the progress callback of
  `add github:` and `update` (a temporary of the engine call's statement, so it goes when the call
  returns); while it lives, `emit` drops a line whose pipe is closed and notes it, and the guard's drop
  ends the run with exit code 0 once the addon's install returned. `add <file.zip> --dry-run` reports the
  sweep's warnings with its own.
- 2026-10-22 — tests: `tests/install.rs` (a stale folder is gone after the next install while a locked one
  and a young one stay; the run's own download folder survives a sweep with the clock two hours ahead during
  the install; a folder with `previous\` stays; a leftover that cannot be removed is a warning and the
  install stands, Windows), `tests/cli.rs` (`add github:` into a pipe closed before the first write installs
  the addon, exits 0 with nothing on stderr and leaves `staging\` empty).
- 2026-10-22 — docs: `docs/development.md` gains the paragraph on the staging folder's lifetime after the
  one install path, the layout entry of `staging`, the `staging\` line of the home folder, and the
  in-flight rule after the closed-pipe paragraph of 075.

## Deviations from the plan

- The run folder and the sweep live in a new module, `mrgl_core::staging`, instead of inside `install.rs`;
  `RunDir` replaces `fresh_staging_dir` and `RemoveOnDrop` there.
- A folder that holds `previous\` is never swept, a rule the plan did not name: it protects the folders
  a failed revert or an interrupted swap moved aside.
- The closed-pipe check is a new test of an install (`add github:` into a closed pipe) next to the 075
  test, since `list` stages nothing.
- `CLAUDE.md` is unchanged: the lock file sits inside each run folder, and the `staging\` entry it names
  stays as it is.

## Verification

- 2026-10-22 — before the change, the new CLI test failed: `mrgl add github:Questie/Questie` into a pipe
  closed before its first write exited 0 at the `resolving` line with nothing installed. After it, the
  test passes: the addon is installed, the sidecar names it, `staging\` is empty.
- 2026-10-22 — by hand in Git Bash against the fixtures (a temporary game folder, `MRGL_HOME` in a
  temporary folder, `MRGL_HTTP_STUB` on `tests/fixtures/releases`): `mrgl --serious add
  github:Questie/Questie --flavor forever | head -N` for N = 1, 2, 3, ten rounds each. The exe built
  before the change left a folder under `staging\` and no addon in 2 of 30 runs; the exe of this change
  left nothing and installed the addon in 30 of 30, every run with exit code 0. A planted download folder
  named two hours old was gone after one such run.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (`mrgl-core` lib 196, `tests/install.rs` 40, `tests/cli.rs` 128, every other suite green),
  `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check` green. The
  `cfg(not(windows))` side (the `flock` behind `File::try_lock`) is compiled by CI's `non-windows` job.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
