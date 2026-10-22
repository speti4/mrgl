---
status: idea
size: S
origin: done/009-tauri-shell-with-the-flavor-list.md
created: 2026-10-22
milestone: M1 Core + CLI
---

# The dependency check counts the client's own `Blizzard_*` addons as present

What: a `## Dependencies` or `## RequiredDeps` entry that names a `Blizzard_*` addon is met by the client
itself, so the dependency check (029) no longer lists it as missing in `list`, after `add` or in the window.
Why: on a real retail install every addon that depends on a Blizzard addon showed as broken.
Done when: a fixture addon that requires `Blizzard_Something` reports no missing dependency, and a missing
non-Blizzard dependency is still reported.
