---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M2 Window MVP
---

# Game-running detection: no `WTF` write while the client runs

What: the core finds a running process whose executable lives under the flavor's folder (by path, not by
name: the Forever executable's name is unknown before launch; a process crate or the Windows API, with the
one-line reason D-002 asks for in `Cargo.toml`), and every write under `WTF` (a backup restore, an addon set, a
SavedVariables cleanup) refuses while it runs; an addon install only warns (the client reads addons at
`/reload`). The voice gets a line for both. Decided 2026-10-21: M2, before `mrgl doctor` (024), so the doctor's
report and the install warning are in the first release; 013, 014 and 025 depend on it.
Why: the client rewrites `AddOns.txt` on logout and holds `WTF` files open; a write during a session is lost or
corrupts a file.
Done when: a test process started from a fixture flavor folder is detected, the `WTF` writes refuse with the
documented message, and the check costs nothing when no process runs.
