---
size: S
origin: done/028-lists-as-discovery-and-forever-starter-list.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 065 — The install engine names a multi-folder addon after the zip or the repository, not the first folder

## Context

When no folder of a zip is extended by all the others, `install::derive_name` prefers the folder named
after the zip's stem or the source repository before falling back to the first folder by name. The DBM-Core
zip is recorded as `DBM-Brawlers`, so `mrgl add github:DeadlyBossMods/DeadlyBossMods` installs under a name
nobody looks for.
Done when: a fixture zip shaped like DBM-Core is recorded under its core folder's name, and the existing
naming tests are unchanged.

Decided 2026-10-22: second of the chain 064 → 065 → 066 → 067 on one branch.

## Docs that move

- `docs/development.md`: the naming rule of a multi-folder addon, where it is stated.

## Not in this change

- Renaming an addon already recorded in a sidecar under the old name; the `## Title` of a TOC as a name
  source.

## Steps

1. [ ] `install.rs`: `derive_name` takes the zip's stem and the repository name as hints and prefers a folder
       that matches one of them (case-insensitive, with the flavor suffix and version stripped from the stem)
       before the alphabetical fallback; `Engine::stage` passes the hints
2. [ ] Tests: a fixture zip shaped like DBM-Core (`DBM-Brawlers`, `DBM-Core`, `DBM-GUI`, …) is recorded as
       `DBM-Core`; the existing naming tests unchanged
3. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; second of the chain 064 → 065 → 066 → 067

## Deviations from the plan

- none yet

## Verification

- (dated, concrete: what was run, what it showed)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
