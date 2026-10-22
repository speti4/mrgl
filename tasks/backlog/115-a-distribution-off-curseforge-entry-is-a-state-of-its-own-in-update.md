---
status: idea
size: S
origin: wip/096-the-curseforge-source-with-the-users-key.md
created: 2026-10-22
milestone: M4 Sources
---

# A distribution-off CurseForge entry is a state of its own in the update check

What: a `curse:` entry whose project allows no download outside CurseForge (`allowModDistribution: false`)
gets its own state in the update check and the window's row, as the pending state of 104, counted apart
from the failed checks: the line names the page and the zip-in-hand way (`add <zip> --as curse:<id>`), and a
run with such entries only ends with exit code 0.
Why: 096 reports it as a failed check, which turns every run red for a choice the author made, not an error.
Done when: the distribution-off fixture (id 4244) ends an `update --all` with exit code 0 and its own line
and JSON state, the window's row shows it, and a failed check still ends with 1.
