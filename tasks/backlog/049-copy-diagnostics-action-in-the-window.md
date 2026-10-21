---
status: idea
size: S
origin: done/024-doctor-diagnostic-report.md
created: 2026-10-21
milestone: M2 Window MVP
---

# "Copy diagnostics" in the window: the doctor report on the clipboard

What: a window action that builds the doctor report of 024 through the core call the CLI uses and puts
its plain text on the clipboard, with a toast through the voice; the same text `mrgl doctor` prints.
Why: a bug report from the window should carry the same block as one from the command line, without a
terminal.
Done when: the action copies the report text, the toast says so, and the copied block matches
`mrgl doctor` on the same folder.
