---
status: idea
size: S
origin: done/078-the-windows-cached-view-walks-the-releases-and-keeps-the-last-answer.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The command line's update check writes the last-check file the window reads

What: `mrgl update --all` and `mrgl update <name>` record their check in `<flavor-id>\last-check.json`
through `mrgl_core::last_check` as the window's check does (078), so a source gone, a failed check or a
budget stop found on the command line shows in the window, and a source found back clears it there.
Why: the file is written by the window only, so after `mrgl update` finds a gone source back while the
version stays the same, the window still shows it gone until its own next check.
Done when: a CLI test's check writes the file with the kept states, a second check that finds the source
back clears the entry, and the window's flavor list test reads what the command line wrote.
