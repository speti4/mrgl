---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 001 — Project scaffolding: repo structure, task standard, decisions, origin and research

## Context

Greenfield repo. Planning on 2026-10-21 settled the frame: build instead of adopt (D-001), Rust + Tauri 2
with a CurseForge-aware but not CurseForge-dependent source set (D-002), a state sidecar under
`%LOCALAPPDATA%\mrgl\` (D-003), a public repo from the first commit with the full CI matrix (D-004) and the
murloc voice (D-005). Decided 2026-10-21: the scope is a toolbox (WTF backups, addon sets) with its own
milestone, M3, after the window and before the other sources. This task lays down the repo structure, the task standard, the decisions, the origin
and research pages and an empty Cargo workspace that compiles, tests and passes every CI check; no product
code beyond the voice table.

## Docs that move

- `README.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`: written for mrgl after
  a proven repo pattern.
- `docs/origin.md`, `docs/research/2026-10-21-addon-manager-alternatives.md`, `docs/nerglish.md`,
  `docs/development.md`, `docs/git-workflow.md`, `docs/maintainer-sessions.md`: new.
- `tasks/roadmap.md`, `tasks/decisions/D-001…D-005`: new.
- `CHANGELOG.md`: the Unreleased section.

## Not in this change

- Any product code: flavor detection, the scan, the sources, the install engine and the sidecar are tasks
  002–004 and 006; the window's list is M2.
- The task-to-issue mirror workflow (016) and the macOS bundle (M6); `scripts/sync-issues.py` is in the
  repo only because the changelog generator imports its parser.
- A usage guide in the zip and signed exes (M5).
- The GitHub rulesets and CodeQL, which are repository settings made by hand after the first push
  (`docs/development.md`).

## Steps

1. [x] Project registered in the maintainer's planning tool
2. [x] `docs/origin.md`, `docs/research/2026-10-21-addon-manager-alternatives.md`, `docs/nerglish.md`
3. [x] `tasks/` standard: roadmap, D-001…D-005, backlog 002–017, this file, generated README
4. [x] Repo files after a proven pattern: community files, docs, scripts, hooks, CI, dependabot, templates
5. [x] Cargo workspace: `mrgl-core` (module stubs, the voice table with tests), `mrgl-cli` (`mrgl`, the
       clap skeleton with end-to-end tests), `mrgl-app` (the Tauri 2 shell, a Node test for the UI voice)
6. [x] Every check green locally: fmt, clippy, test, msrv, deny, zizmor, changelog, commit-rules self-test,
       `tasks_check`
7. [ ] The public repo `speti4/mrgl`, the first push, CI green

## Log

- 2026-10-21 — `mrgl` free on crates.io and GitHub (both 404); folders created, the reusable files of the
  pattern copied and rewritten for mrgl
- 2026-10-21 — D-001…D-005 decided; backlog 002–017 reviewed one by one
- 2026-10-21 — review of the scaffold before the first commit, over four areas (code, CI and scripts, docs,
  tasks): the public-text rules applied throughout (no private path or machine detail, decisions and results
  instead of conversation, no timing constraints), the research page's claims checked against their sources,
  "one portable zip with two exes" and the `WTF` write scope stated the same way in every file, American
  spelling; in the code, `--help` plain under `--serious`, the window honors `MRGL_SERIOUS`, tests assert
  against the voice table rather than the murloc vocabulary, the JSON `kind` has a fixed name, the UI smoke
  test runs every Node test and fails in CI without Node, the hook files are executable, the commit-rules job
  checks every commit of a first push and keeps the Markdown headings of a pull request text
- 2026-10-21 — every local check green: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D
  warnings`, `cargo test --locked` (15 tests, the Node UI tests included), `cargo +1.90 check --all-targets
  --locked`, `cargo deny check`, `uvx zizmor .github` (no findings), `python -I scripts/changelog.py --check`,
  `sh scripts/check-commit.sh --self-test` (38 cases), `tasks_check` 0 errors; the window exe starts and the
  CLI prints the two-line banner

## Deviations from the plan

- none yet

## Verification

- (to be written at the close)

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
