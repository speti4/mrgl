---
status: idea
size: S
origin: done/015-list-export-import-and-interface-warning.md
created: 2026-10-21
milestone: M4 Sources
---

# What the Wago, WoWInterface and CurseForge sources owe the existing features

What: once a new source lands, three features of M1 and M2 take it on the same footing as GitHub: the
list import (015) resolves and installs its entries instead of reporting them as not importable; the
doctor (024) names the source's API key by name, present or absent, never its value; and the Forever
build signal (011, yes, no or unknown) is answered from what the source's API says about the game
versions a file supports, on the pick and in `list`.
Why: an exported retail list is mostly CurseForge and Wago entries, the first CurseForge bug report asks
"is the key there", and a Forever user wants the same signal whatever the source.
Done when: a fixture list with one entry per source imports every entry against the sources' stubs, the
key names are in the doctor's list, and a fixture answer per source classifies the Forever build offline.
