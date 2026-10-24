---
size: S
origin: wip/096-the-curseforge-source-with-the-users-key.md
created: 2026-10-22
started: 2026-10-24
plan: none
milestone: M2 Window MVP
---

# 116 — The window installs a zip in hand with a recorded source

## Context

The window's add action takes a zip from disk and a source spec to record with it, the pair of the command
line's `add <zip> --as <spec>` (096), so a distribution-off CurseForge addon downloaded from its page is
installed and managed from the window; the row then carries the source like any other.
Why: 096 gave the command line the zip-in-hand way for a project that allows no download outside CurseForge,
and the window has no way to do the same.
Done when: a window test installs the 096 zip in hand with `curse:<id>` recorded, the row shows the source, and
the next update check asks that source.

Decided 2026-10-24: the third of three follow-ups on one branch (114, 115, 116).

## Docs that move

- `docs/development.md`: the window's add action and its commands, where it lists them
- `README.md`: the window's paragraph, if it lists what the window installs

## Not in this change

- A window field for the CurseForge key; fingerprint matching
- The command line (096 did `add <zip> --as <spec>`)

## Steps

1. [ ] `crates/mrgl-app/src/actions.rs` (and the command it backs): the add action takes a zip path and an
       optional source spec, recorded with the install as the command line's `--as` does (any spec but a Wago
       page address; the version is the TOC's)
2. [ ] The UI: the add form offers the source spec next to the zip pick; the row shows the recorded source
       with its badge like any other
3. [ ] Tests: a window test installs `MrglNoDistribution-2.0.0.zip` with `curse:4244` recorded, the row shows
       the source, the next update check asks `curse:4244`; the Node tests of the UI modules

## Log

- 2026-10-24 — promoted

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
