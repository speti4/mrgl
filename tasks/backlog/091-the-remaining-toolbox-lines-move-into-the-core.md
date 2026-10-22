---
status: idea
size: S
origin: done/086-one-set-and-savedvars-summary-in-the-core.md
created: 2026-10-22
milestone: M3 Toolbox
changelog: housekeeping
---

# The remaining toolbox lines move into the core: the set list note, the not-installed warning and the two-account error's words

What: the set list note (`2 sets of _forever_ in …`) and the apply's not-installed warning move into the
core as the summaries of 086 did, with the command line and the window's toolbox (085) both printing them
from there; and the core's two-account error names the account to pick without the command line's
`--account` flag, so the window can show the core's words.
Why: 086 moved the three summaries and left these two lines in both callers, and the two-account error ends
with a flag the window never has.
Done when: one function per line in the core with both callers, the CLI and the window tests assert the same
text, and the two-account error reads the same from both without naming a flag.
