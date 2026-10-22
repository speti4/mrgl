---
status: idea
size: M
origin: done/014-addon-sets-with-enable-and-disable.md
created: 2026-10-22
milestone: M3 Toolbox
---

# The window's toolbox: a set switch and a SavedVariables view

What: per flavor, a set picker that applies a set to every character or to one (`sets::apply_set` with a
`Target`), and a SavedVariables view with sizes, orphans and a cleanup button (`savedvars`), both refused
while the client runs with the voice's line.
Why: the toolbox commands of M3 exist in the CLI only; the window is where most users will look for them.
Done when: on a fixture flavor the window applies a set and moves the orphans into the dated zip, and both
are refused with the stand-in client running.
