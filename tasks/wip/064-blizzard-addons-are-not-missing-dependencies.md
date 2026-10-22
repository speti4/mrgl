---
size: S
origin: done/009-tauri-shell-with-the-flavor-list.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 064 — The dependency check counts the client's own `Blizzard_*` addons as present

## Context

A `## Dependencies` or `## RequiredDeps` entry that names a `Blizzard_*` addon is met by the client itself,
so the dependency check (029) no longer lists it as missing in `list`, after `add` or in the window. On a
real retail install every addon that depends on a Blizzard addon showed as broken.
Done when: a fixture addon that requires `Blizzard_Something` reports no missing dependency, and a missing
non-Blizzard dependency is still reported.

Decided 2026-10-22: first of the chain 064 → 065 → 066 → 067 on one branch, the M1 fixes found on the real
install, done before the pre-launch manual round (045).

## Docs that move

- `docs/development.md`: the dependency check's rule gains the `Blizzard_` line, if the rule is written there.

## Not in this change

- An embedded library as a required dependency (029's rule, checked on the client in 045); the optional
  dependencies; the window's row text (009 shows `missing` as the core reports it).

## Steps

1. [ ] `dependencies.rs`: a required dependency whose name starts with `Blizzard_` (ASCII case-insensitive)
       counts as met by the client, before the folder lookup
2. [ ] Tests: a fixture addon requiring `Blizzard_Something` has no missing dependency; a missing
       non-Blizzard one is still reported; `list`, `add` and the window rows agree
3. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 064 → 065 → 066 → 067

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
