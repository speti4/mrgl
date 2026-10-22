---
status: idea
size: S
origin: done/109-the-windows-adopt-view.md
created: 2026-10-22
milestone: M2 Window MVP
changelog: housekeeping
---

# The toolbox's adopt lines in the window's words, without the command line's flags

What: the core's adopt summary, marker line and refusal take a caller's wording, as `CleanupBy` does for the
SavedVariables summary (086), so the toolbox's adopt card says "the Adopt button adopts N of them" and names
its confirmation instead of `adopt --apply` and `--alongside-curseforge`; the command line keeps its flags.
Why: 109 shows the core's lines as they are, and three of them name command-line flags the window has not.
Done when: the card's three lines name no flag, the command line's are unchanged, and the Rust and Node tests
assert both wordings.
