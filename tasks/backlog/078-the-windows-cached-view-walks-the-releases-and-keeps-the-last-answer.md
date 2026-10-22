---
status: idea
size: S
origin: done/066-github-source-looks-past-the-newest-release.md
created: 2026-10-22
milestone: M2 Window MVP
---

# The window's cached view: the Forever build signal walks the releases like the resolve, and the last online answer is kept

What: `GitHub::cached_forever_build` (and any cache-only compare left in the window) takes the newest cached
release that has an asset for the flavor, as the resolve does since 066, instead of the channel's newest
release; and the window keeps the answer of its last online check (a source gone by a 404, a failed check,
the budget stop) across a redraw caused by a mark or a restart, until the next check replaces it.
Why: a repository whose newest release lacks a Forever zip shows a Forever build of `no` and an update the
resolve would not install; and the window forgets what the check found as soon as a row is marked (010).
Done when: the fixture repository of 066 shows the Forever build of the release before the newest in the
window's offline list, and a flavor list test marks a row after a check that reported a source gone and still
shows it gone.
