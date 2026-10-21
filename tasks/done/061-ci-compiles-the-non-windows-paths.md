---
size: S
origin: done/005-dev-addon-linking-with-junctions.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
closed: 2026-10-22
---

# 061 — CI compiles the non-Windows paths of the core

## Context

A CI job on the Linux runner runs `cargo check --all-targets` (or the core's tests) so the
`cfg(not(windows))` branches of `link`, `client` and `discover` compile; today the Linux job only runs
`fmt` and the changelog check, and a broken non-Windows path is found at M6. The macOS preview (M6) starts
from a core that compiles there; cheap to keep true from now.
Done when: the job runs on every pull request and a deliberate non-Windows compile error fails it.

Decided 2026-10-22: paired with 033 on one branch (both are CI and release housekeeping); the check is
`cargo check --all-targets --locked` on `ubuntu-latest` for the workspace, the Tauri crate included only
if its Linux system dependencies install in under a minute, otherwise `-p mrgl-core -p mrgl-cli`.

## Docs that move

- `docs/development.md` ("CI and releases"): the new job and what it proves; `CONTRIBUTING.md` if its
  check list names the jobs.

## Not in this change

- A macOS job (036, M6); running the tests on Linux (the fixtures are Windows-shaped today).

## Steps

1. [x] The job in `.github/workflows/ci.yml` in the house style (pinned actions, the toolchain action,
       the cache); `uvx zizmor .github` clean
2. [x] The non-Windows paths fixed if the first run finds them broken (the fix is this task's)
3. [x] The job added to the required checks is the main session's step after the merge; say so in
       Verification
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), paired
  with 033
- 2026-10-22 — the job `non-windows` in `.github/workflows/ci.yml`: `ubuntu-latest`, the pinned checkout
  with `persist-credentials: false`, the local toolchain action, an apt step for the window's Linux
  libraries, then `cargo check --all-targets --locked` over the whole workspace. The release job now needs
  it like the other five jobs. The apt packages come from the Linux graph in `Cargo.lock`
  (`cargo tree --target x86_64-unknown-linux-gnu -i <crate>`): the `-sys` crates found through pkg-config
  are GTK 3, WebKitGTK 4.1, libsoup 3, JavaScriptCore (all brought by `libwebkit2gtk-4.1-dev`) and D-Bus
  (`dbus` of `tao`, `libdbus-1-dev`); `libappindicator` is not in the graph of the enabled features, and no
  `rsvg` or `xdo` crate is. `uvx zizmor .github`: no findings.
- 2026-10-22 — the first run (dispatched on the branch, run 39687525301): every job green; `non-windows`
  took 2 min 14 s, of which the apt step 35 s and the check 85 s, without a Rust cache. The install stays
  under a minute, so `mrgl-app` is in the check. No non-Windows path was broken: the `cfg(not(windows))`
  branches of `link`, `client`, `discover` and `backup` and the non-Windows tests of `mrgl-core` and
  `mrgl-cli` compiled as they were.
- 2026-10-22 — the failure proof: a throwaway commit put a type error into the `cfg(not(windows))`
  `create_junction` of `link.rs` (a `&str` bound to a `u32`); a local `cargo check -p mrgl-core` on Windows
  passed with it, and the dispatched run 39687868855 failed `non-windows` with `error[E0308]: mismatched
  types --> crates/mrgl-core/src/link.rs:607:32` and exit code 101. The commit was then removed from the
  branch (reset and `--force-with-lease`), so the pull request does not carry it.
- 2026-10-22 — docs: `docs/development.md` ("CI and releases") names the sixth job in the job list, a
  paragraph says what it compiles, why it checks and does not test, the apt packages and their measured
  time, and the fallback to `-p mrgl-core -p mrgl-cli`; the ruleset's required checks and "The release"
  count the job. `docs/git-workflow.md` lists `non-windows` among the required checks. `CONTRIBUTING.md`
  does not name the jobs in its check list; one sentence after it says CI runs the `non-windows` check that a
  Windows machine cannot.

## Deviations from the plan

- No Rust cache, though step 1 names the cache: the house style keeps zizmor's accepted cache-poisoning
  finding on the one cache step of the `test` job, and the `msrv` job builds without a cache for the same
  reason; at about two minutes a run, the cache would buy little.
- No local `cargo check --target x86_64-unknown-linux-gnu`: from Windows it needs the Linux target and a
  Linux C compiler for `ring`; the failure was proven in CI with a throwaway commit instead.
- `docs/git-workflow.md` changed too: it lists the required checks of the ruleset.

## Verification

- 2026-10-22 — the dispatched run 39687525301 on the branch: `non-windows` green in 2 min 14 s, with
  `mrgl-app` in the check (apt 35 s); the throwaway run 39687868855 failed `non-windows` on the deliberate
  error in the `cfg(not(windows))` branch of `link.rs`, while the same code passed `cargo check` on Windows.
  Locally: `cargo fmt --check`,
  `python -I scripts/changelog.py --check` and `uvx zizmor .github` (no findings) pass; no Rust code changed.
- After the merge, the main session adds `non-windows` to the required checks of the ruleset on `main`
  (the docs already list it there); until then the job runs on every pull request without blocking it.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
