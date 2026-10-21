---
size: S
origin: done/003-github-release-source-with-release-json.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 042 — GitHub source: pagination past 30 releases, the rate-limit reset time, `release.json` cached per tag

## Context

Three edges of the GitHub source (003): the release list follows the `Link` header past the first
page of 30 when the first page holds no stable release; a 403 or 429 hint names the time the rate limit
resets (`x-ratelimit-reset`) and the token variable; and a release's `release.json` is cached per tag next
to the release list, so a repeated resolve costs one conditional request, not two. A repository that
publishes nightly prereleases pushes the stable release off the first page, and a user without a token
hits the hourly limit on an `update --all` over a long list.
Done when: a fixture repository with 35 releases resolves the stable one on page two, the 403 hint carries
the reset time, and a second resolve of the same tag reads `release.json` from the cache (the stub's log
shows one request).

Decided 2026-10-22: first of the chain 042 → 046 → 041 on one branch, all three in `sources/github.rs`.

## Docs that move

- `docs/development.md` (the source and its cache under the home folder): the per-tag cache file and the
  page walk.

## Not in this change

- The asset download's error path and size check (046, next in the chain); the offline resolve (041);
  any other source.

## Steps

1. [ ] The release list follows `Link: rel="next"` while no stable release was found, with a page bound
2. [ ] A 403 or 429 answer renders the reset time from `x-ratelimit-reset` (local time) and names
       `MRGL_GITHUB_TOKEN` in the hint
3. [ ] `release.json` cached per tag next to the release list under the home folder, read before the
       network on a repeated resolve
4. [ ] Tests on the stub: 35 fixture releases with the stable one on page two; the 403 hint; one request on
       the second resolve of a tag
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first of
  the chain 042 → 046 → 041

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
