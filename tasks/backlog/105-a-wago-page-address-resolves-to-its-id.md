---
status: idea
size: S
origin: done/095-the-wago-source.md
created: 2026-10-22
milestone: M4 Sources
---

# A Wago page address resolves to its `wago:<id>` through the keyless slugs list

What: `add https://addons.wago.io/addons/<slug>` (and the same address in a list) resolves the slug to the
addon's id through `Wago::id_of_slug` over the keyless slugs endpoint of the flavor's game version, and the
sidecar records `wago:<id>`; a slug the list does not carry says so.
Why: 092 refuses the Wago page address with a hint, since it names a slug, not an id; 095 brought the slugs
list that answers it without a key.
Done when: the page address of a fixture slug installs from the stub as `wago:<id>`, an unknown slug names
the game version it was looked up in, and a list entry with the address imports.
