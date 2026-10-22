---
status: idea
size: S
origin: done/042-github-source-pagination-rate-limit-and-per-tag-cache.md
created: 2026-10-22
milestone: M1 Core + CLI
changelog: housekeeping
---

# GitHub cache housekeeping: stale per-tag `release.json` files and the old single file

What: the per-tag `release.json` cache (042) drops the files of tags the cached release list no longer holds,
and the old `release-json.json` left in existing home folders is removed on the first resolve.
Why: the cache grows by one file per release forever, and the old file is dead weight nobody reads.
Done when: a fixture cache with stale tags and the old file is cleaned on a resolve, and a current tag's file
stays.
