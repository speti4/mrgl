---
status: idea
size: S
origin: done/055-one-core-helper-for-resolve-download-and-install.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The link guard also covers the folders a new version drops

What: `Engine::commit` checks every folder of the previous version that the new one no longer has before it
moves it aside, so a dropped folder that has become a junction is refused like a staged one.
Why: today only the staged folders are checked; a stale folder turned junction into a development folder is
moved aside and deleted with the staging folder.
Done when: a fixture update whose old version has a folder now linked by `mrgl link` is refused and leaves the
development folder untouched.
