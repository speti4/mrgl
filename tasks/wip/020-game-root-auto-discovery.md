---
size: S
origin: wip/002-flavor-detection-and-addons-scan-with-toc-parsing.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M2 Window MVP
---

# 020 — Game root auto-discovery: the launcher's registry key and the common install folders

## Context

The first run of the command line and the window should find the game without a flag; task 002 settled for
`--root`, `MRGL_ROOT` and `mrgl root <folder>`. When none of those names the game folder, `home.rs` (or a
`discover` module) looks it up in the launcher's registry key (`HKLM\SOFTWARE\WOW6432Node\Blizzard
Entertainment\World of Warcraft`, `InstallPath`, through `windows-sys`'s registry functions, already a
dependency since 023) and in the usual folders (`C:\Program Files (x86)\World of Warcraft`, the same under
`Program Files`), checks that the hit is a game root (`.build.info`, at least one flavor folder), and
offers it: `mrgl list --flavor forever` works with no flag, `mrgl root` with no argument prints the hit and
says where it came from, and `mrgl root <folder>` saves it as before. The lookup sits behind a trait with a
stub so the tests stay machine-independent; the window's folder picker (009) starts from the same hit.
Done when: a machine with the launcher's key finds its root with `mrgl list --flavor forever` alone, the
lookup is stubbed in the tests, and the window's folder picker (009) can start from the same hit.

Decided 2026-10-21: discovery never writes `config.json` on its own; a discovered root is used for the run
and reported, and `mrgl root <folder>` (or the window's picker) is the one place that remembers it.

## Docs that move

- `docs/development.md` ("Fixtures" and the paragraph on `config.json` / `MRGL_ROOT`): the discovery order.
- `README.md`: the first-run line (no flag needed on a standard install).

## Not in this change

- The window's folder picker itself (009); only the core call it will use.
- A macOS lookup (`/Applications/World of Warcraft/`, M6).

## Steps

1. [ ] The discovery trait (registry value, candidate folders) with the Windows implementation and the stub
2. [ ] The resolution order in the CLI: `--root`, `MRGL_ROOT`, the remembered root, the discovery; the
       voice line that says where the root came from
3. [ ] `mrgl root` with no argument reports the hit without saving; tests with the stub for every order
4. [ ] Docs; every check green; close

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
