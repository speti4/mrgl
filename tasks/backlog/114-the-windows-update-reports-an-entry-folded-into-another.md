---
status: idea
size: S
origin: done/108-update-all-over-two-entries-of-one-source-whose-first-install-takes-the-second-folder.md
created: 2026-10-22
milestone: M2 Window MVP
---

# The window's update reports an entry folded into another during the run

What: the window's update action reads `Installed::folded` of the core (108) and reports an entry an
earlier install of the run folded into its addon with the core's line, as its own phase on the row, and
counts it as `folded` in the summary instead of passing `folded: 0`.
Why: 108 gave `update --all` the outcome and the line; the window still counts and says nothing about an
entry the run dropped.
Done when: a window test over the 108 fixture (two entries of one source, a zip with both folders) shows
the folded line on the second row and the summary counts one folded, and the sidecar holds one entry after.
