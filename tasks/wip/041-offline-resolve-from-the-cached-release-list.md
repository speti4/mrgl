---
size: S
origin: done/003-github-release-source-with-release-json.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 041 — Offline resolve from the cached release list, with a warning

## Context

When the GitHub releases API cannot be reached (no network, a timeout) and the repository's release
list is in the cache of 003, the source resolves from the cached list and the voice warns that the pick
may be stale (the cache's date in the line); a cache miss stays the network error it is today. A resolve
on a flaky connection should not fail when yesterday's answer is at hand; `update --all` (006) over
twenty addons wants the same.
Done when: with the stub returning a network error and a warm cache, `add --dry-run` reports the pick with
the warning, and with a cold cache the network error.

Decided 2026-10-22: third of the chain 042 → 046 → 041; the per-tag `release.json` cache of 042 serves
the offline resolve too, so a stale pick needs no network at all.

## Docs that move

- `docs/development.md` (the source and its cache): the offline rule and the warning line.

## Not in this change

- An offline download (the asset is never cached before install); the window's offline state (009).

## Steps

1. [ ] A network error with a warm release-list cache resolves from the cache; the pick carries a stale
       flag with the cache's date; a cold cache keeps the network error
2. [ ] The voice's warning line on `add --dry-run`, `add` and the update check, plain and `--json`
3. [ ] Tests on the stub: warm cache plus network error, cold cache plus network error, a warm cache with
       a working network (no warning)
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), third
  of the chain 042 → 046 → 041

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, at close)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
