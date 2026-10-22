---
status: idea
size: S
origin: done/076-the-checks-say-what-the-client-says.md
created: 2026-10-22
milestone: M2 Window MVP
---

# The window's rows show the interface states `scan` and `list` mark

What: a row of the flavor list carries the addon's interface state, `interface mismatch` (015) or `out of
date` (076), from `interface::check_addon` and `interface::out_of_date_addon`, drawn as the command line
marks its lines, with the warning's words in the row's title.
Why: `scan` and `list` mark both states since 076 while the window shows neither, so an addon the client
lists as `Out of date` looks fine in the window.
Done when: a flavor list test over the `interface/` fixture sees the three rows (current, behind, another
expansion) with their states, and the mock data shows one of each.
