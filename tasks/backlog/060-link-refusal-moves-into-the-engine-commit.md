---
status: idea
size: S
origin: done/005-dev-addon-linking-with-junctions.md
created: 2026-10-21
milestone: M3 Toolbox
---

# The linked-addon refusal moves into the engine's commit

What: the refusal to install over a linked addon or a foreign junction (005, `refuse_links`) moves from
`Engine::install` into `Engine::commit`, so every path that stages and commits (the pick install of 040,
`import` of 015, `update` of 006, the rollback of 022) is covered by the one call and none has to call
it itself.
Why: a caller that forgets the check overwrites a junction into the user's development folder.
Done when: the check runs once in `commit`, the callers' own calls are gone, and the 005 tests pass.
