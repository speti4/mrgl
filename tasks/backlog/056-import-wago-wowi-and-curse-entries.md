---
status: idea
size: S
origin: done/015-list-export-import-and-interface-warning.md
created: 2026-10-21
milestone: M4 Sources
---

# `import` installs `wago:`, `wowi:` and `curse:` entries once those sources land

What: the list import (015) resolves and installs an entry of each new source the same way it does a
`github:` one today, instead of reporting it as not importable yet.
Why: an exported list from a retail folder is mostly CurseForge and Wago entries; an import that skips
them is half an import.
Done when: a fixture list with one entry per source imports every entry against the sources' stubs.
