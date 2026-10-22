---
status: idea
size: S
origin: done/089-update-one-reports-a-second-entry-of-the-same-source.md
created: 2026-10-22
milestone: M2 Window MVP
---

# The window's update reports a second entry of the same source that has the pick's version

What: the window's update action (`crates/mrgl-app/src/actions.rs`) treats the "installed already" answer of
the one install path as its own outcome, naming the other entry of the same source, and counts it as
`installed_already` in the core summary of 089, as the command line does; the row's message says so.
Why: the window still counts that answer as updated and shows no line about the entry it found, the
behavior 089 fixed on the command line.
Done when: a window test over a sidecar with two entries from one source shows the line and counts the
addon as installed already, not updated, and the summary line reads as the command line's.
