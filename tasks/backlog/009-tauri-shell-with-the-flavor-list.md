---
status: idea
size: M
origin: session 2026-10-21
created: 2026-10-21
milestone: M2 Window MVP
---

# Tauri shell with the flavor list

What: on the layout of the design sketch (017), the window finds the game folders (a chosen root,
remembered) and fills the list per flavor with each addon's name, version, source badge and state (managed,
unmanaged, linked, update available) from the core, with search. The IPC data module implements the
contract of the sketch's `mock-data.js` behind `window.Data` and replaces it in `index.html` (the mock stays
for a browser look); the core's `Message` gains a `kind` field so the window picks the voice placement;
a resolve through IPC reuses the source's `Pick` (003), never a second code path.
Why: the daily overview the CurseForge app gives today, without it.
Done when: the four flavors of a real install show their addons with the right states, offline.
