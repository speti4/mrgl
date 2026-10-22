---
size: S
origin: done/066-github-source-looks-past-the-newest-release.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 078 — The window's cached view: the Forever build signal walks the releases like the resolve, and the last online answer is kept

## Context

`GitHub::cached_forever_build` (and any cache-only compare left in the window) takes the newest cached
release that has an asset for the flavor, as the resolve does since 066, instead of the channel's newest
release; and the window keeps the answer of its last online check (a source gone by a 404, a failed check,
the budget stop) across a redraw caused by a mark or a restart, until the next check replaces it. A
repository whose newest release lacks a Forever zip shows a Forever build of `no` and an update the
resolve would not install; and the window forgets what the check found as soon as a row is marked (010).
Done when: the fixture repository of 066 shows the Forever build of the release before the newest in the
window's offline list, and a flavor list test marks a row after a check that reported a source gone and still
shows it gone.

Decided 2026-10-22: first of the chain 078 → 079 on one branch, in wave 6.

## Docs that move

- `docs/development.md`: the `flavors` command's offline rule (the cached Forever build walks the releases,
  the last online answer kept) and the GitHub source's cached signal.

## Not in this change

- A new online request from the list; the CLI's `list` (its Forever build comes from the same core call and
  follows); the checks after an update and the title bar (079).

## Steps

1. [ ] `sources/github.rs`: `cached_forever_build` walks the cached releases of the channel, newest first, to
       the first with an asset for the flavor, as the resolve does; any cache-only compare left in
       `mrgl-app/src/list.rs` follows the same walk
2. [ ] The window keeps the answer of the last online check per addon (source gone, failed, the budget stop)
       across a redraw caused by a mark and across a restart, until the next check replaces it
3. [ ] Tests: the fixture repository of 066 shows the Forever build of the release before the newest in the
       offline list; a flavor list test marks a row after a check that reported a source gone and still shows
       it gone; the stub tests of the source
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 078 → 079

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
