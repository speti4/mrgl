---
status: idea
size: S
origin: done/074-window-toolbox-sets-and-savedvariables.md
created: 2026-10-22
milestone: M3 Toolbox
changelog: housekeeping
---

# One summary line of `set apply` and `savedvars` in the core, shared by the command line and the window

What: the summary lines of `set apply` and of the SavedVariables listing and cleanup move into the core
(`sets` and `savedvars`), and the command line and the window's toolbox (074) both print them from there;
the orphan count agrees with its verb (`1 orphan (…) has no addon folder`, `2 orphans … have`).
Why: 074 copied the command line's summary text into the window, two copies will drift apart, and both say
`1 orphan … have`.
Done when: one function per summary in the core with both callers, the CLI and the window tests assert the
same lines, and the one-orphan line reads `has`.
