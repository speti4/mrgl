---
status: idea
size: S
origin: wip/096-the-curseforge-source-with-the-users-key.md
created: 2026-10-22
milestone: M2 Window MVP
---

# The window installs a zip in hand with a recorded source

What: the window's add action takes a zip from disk and a source spec to record with it, the pair of the
command line's `add <zip> --as <spec>` (096), so a distribution-off CurseForge addon downloaded from its
page is installed and managed from the window; the row then carries the source like any other.
Why: 096 gave the command line the zip-in-hand way for a project that allows no download outside
CurseForge, and the window has no way to do the same.
Done when: a window test installs the 096 zip in hand with `curse:<id>` recorded, the row shows the source,
and the next update check asks that source.
