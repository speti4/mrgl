---
status: idea
size: S
origin: wip/002-flavor-detection-and-addons-scan-with-toc-parsing.md
created: 2026-10-21
milestone: M2 Window MVP
---

# Game root auto-discovery: the launcher's registry key and the common install folders

What: when no `--root`, `MRGL_ROOT` or remembered root names the game folder, look it up in the launcher's
registry key (`HKLM\SOFTWARE\WOW6432Node\Blizzard Entertainment\World of Warcraft`, `InstallPath`) and in
the usual folders (`C:\Program Files (x86)\World of Warcraft`), and offer the hit; `mrgl root` saves it.
Why: the first run of the command line and the window should find the game without a flag; task 002
settled for `--root`, `MRGL_ROOT` and `mrgl root <folder>`.
Done when: a machine with the launcher's key finds its root with `mrgl list --flavor forever` alone, the
lookup is behind a trait with a stub so the tests stay machine-independent, and the window's folder picker
(009) starts from the same hit.
