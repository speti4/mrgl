---
status: idea
size: S
origin: done/023-game-running-detection.md
created: 2026-10-21
milestone: M6 macOS preview
---

# macOS process list for the game-running check, and local time outside Windows

What: the macOS implementation of the process-list trait of 023 (executable paths of running processes,
`proc_listpids` and `proc_pidpath` or `sysctl`), so the `WTF` write guard and the install warning work on a
Mac; on other targets the list stays empty. The backup names and zip timestamps of 013 use local time on
Windows only (UTC elsewhere); the macOS build gets the same through `localtime_r`.
Why: 023 lists nothing outside Windows, and the M6 preview must not write `WTF` while the client runs
there either.
Done when: the stand-in client test of 023 passes on the macOS CI job of M6.
