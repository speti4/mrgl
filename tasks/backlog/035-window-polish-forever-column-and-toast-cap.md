---
status: idea
size: S
origin: done/017-window-design-sketch-before-the-shell.md
created: 2026-10-21
milestone: M2 Window MVP
---

# Window polish: hide the Forever note column without notes, cap the toast stack

What: the Forever note column header is hidden when no row of the flavor carries a note, and the toast
stack keeps only the newest few (a sticky error stays until dismissed); both in the sketch's view code.
Why: an empty column header is noise on three of the four flavors, and a burst of updates must not bury
the list under toasts.
Done when: the retail fixture data shows no Forever column, and ten quick messages leave the capped number
of toasts with the sticky error on top.
