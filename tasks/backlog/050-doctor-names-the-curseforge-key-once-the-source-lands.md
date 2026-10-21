---
status: idea
size: S
origin: done/024-doctor-diagnostic-report.md
created: 2026-10-21
milestone: M4 Sources
---

# The doctor names the CurseForge key once that source lands

What: the doctor's key list (names only, never values) gains the CurseForge API key's variable when
the CurseForge source joins, so a bug report shows whether the key is set.
Why: the first CurseForge question in a bug report is "is the key there"; the doctor answers it the
same way it answers for the GitHub token.
Done when: the key's name is in the list, present or absent, and no value ever appears in the output.
