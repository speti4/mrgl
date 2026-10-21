---
status: idea
size: S
origin: done/003-github-release-source-with-release-json.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# Offline resolve from the cached release list, with a warning

What: when the GitHub releases API cannot be reached (no network, a timeout) and the repository's release
list is in the cache of 003, the source resolves from the cached list and the voice warns that the pick
may be stale (the cache's date in the line); a cache miss stays the network error it is today.
Why: a resolve on a flaky connection should not fail when yesterday's answer is at hand; `update --all`
(006) over twenty addons wants the same.
Done when: with the stub returning a network error and a warm cache, `add --dry-run` reports the pick with
the warning, and with a cold cache the network error.
