---
size: S
origin: done/042-github-source-pagination-rate-limit-and-per-tag-cache.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 072 — GitHub cache housekeeping: stale per-tag `release.json` files and the old single file

## Context

The per-tag `release.json` cache (042) drops the files of tags the cached release list no longer holds, and
the old `release-json.json` left in existing home folders is removed on the first resolve. The cache grows
by one file per release forever, and the old file is dead weight nobody reads.
Done when: a fixture cache with stale tags and the old file is cleaned on a resolve, and a current tag's
file stays.

Decided 2026-10-22: second of the chain 071 → 072 on one branch.

## Docs that move

- `docs/development.md`: the cache layout under `%LOCALAPPDATA%\mrgl\`, if the per-tag files are listed.

## Not in this change

- The zip cache under `cache\<addon>\<version>.zip` (its own housekeeping is not asked for); a size bound
  on the cache.

## Steps

1. [ ] `sources/github.rs`: after a fresh release list is written, the `tag-*.json` files of tags the list
       no longer holds are removed; a `release-json.json` next to them is removed when present
2. [ ] Tests against the stub: a fixture cache with stale tags and the old file is cleaned on a resolve,
       a current tag's file stays, an offline resolve (041) removes nothing
3. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; second of the chain 071 → 072

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
