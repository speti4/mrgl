---
status: idea
size: S
origin: done/084-stale-staging-folders-are-swept.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The doctor names a staging folder kept for its `previous\` folders

What: `mrgl doctor` lists every run folder under `<flavor-id>\staging\` that the sweep of 084 keeps because
it holds a `previous\` folder (the addon folders a failed swap moved aside), with its age and size, and says
what to do with it.
Why: after the error of a failed revert nothing points the user to that folder again, and the sweep never
removes it.
Done when: a planted run folder with a `previous\` folder shows in the doctor's text and JSON, and one
without it does not.
