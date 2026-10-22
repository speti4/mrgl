---
status: idea
size: S
origin: done/098-adopt-apply-writes-sidecar-entries-without-a-reinstall.md
created: 2026-10-22
milestone: M2 Window MVP
---

# The window's install paths warn in a folder the CurseForge app manages

What: the window's add and update actions warn in a flavor folder with a `.curseclient` marker, with the
core's line of 098 (the CurseForge app may still update or flag those folders), shown once per run on the
flavor's view, as `add`, `import` and `update` do on the command line.
Why: 098 added the warning to the three command-line paths and left the window's actions without it, so
two managers on one folder is said in one place and not the other.
Done when: a window test over a marked flavor folder shows the warning on add and on update, and an
unmarked folder shows none.
