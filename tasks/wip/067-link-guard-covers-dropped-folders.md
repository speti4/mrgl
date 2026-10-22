---
size: S
origin: done/055-one-core-helper-for-resolve-download-and-install.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 067 — The link guard also covers the folders a new version drops

## Context

`Engine::commit` checks every folder of the previous version that the new one no longer has before it
moves it aside, so a dropped folder that has become a junction is refused like a staged one. Today only the
staged folders are checked; a stale folder turned junction into a development folder is moved aside and
deleted with the staging folder.
Done when: a fixture update whose old version has a folder now linked by `mrgl link` is refused and leaves
the development folder untouched.

Decided 2026-10-22: last of the chain 064 → 065 → 066 → 067 on one branch.

## Docs that move

- `docs/development.md`: the link guard's rule, if the engine's commit is described there.

## Not in this change

- The `remove` command's own guard (unchanged); a junction that appears between the check and the swap.

## Steps

1. [ ] `install.rs`: `Engine::commit` computes the stale folders before the guard and passes staged and stale
       folders together to `refuse_links`
2. [ ] Tests: a fixture update whose old version has a folder linked by `mrgl link` is refused, the
       development folder and the AddOns folder untouched; the existing swap and link tests unchanged
3. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; last of the chain 064 → 065 → 066 → 067

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
