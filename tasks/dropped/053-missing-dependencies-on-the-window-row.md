---
size: S
origin: done/029-dependency-check-from-toc-and-nolib.md
created: 2026-10-21
closed: 2026-10-21
reason: folded into 009, the Tauri shell, whose rows carry the addon states from the core
reopen-when: never
milestone: M2 Window MVP
---

# Missing dependencies on the addon's row in the window

What: the window's addon row shows the missing required dependencies the check of 029 reports (the
`dependencies.missing` names), as a short mark with the names on hover or in the detail, through the
voice.
Why: a missing library is the second most common "it does not load"; the window should say it before
the user asks.
Done when: a fixture addon with a missing dependency shows the mark on its row and a met one shows
nothing.
