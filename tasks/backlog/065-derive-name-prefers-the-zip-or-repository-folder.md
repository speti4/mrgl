---
status: idea
size: S
origin: done/028-lists-as-discovery-and-forever-starter-list.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The install engine names a multi-folder addon after the zip or the repository, not the first folder

What: when no folder of a zip is extended by all the others, `install::derive_name` prefers the folder named
after the zip's stem or the source repository before falling back to the first folder by name.
Why: the DBM-Core zip is recorded as `DBM-Brawlers`, so `mrgl add github:DeadlyBossMods/DeadlyBossMods`
installs under a name nobody looks for.
Done when: a fixture zip shaped like DBM-Core is recorded under its core folder's name, and the existing
naming tests are unchanged.
