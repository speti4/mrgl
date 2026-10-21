---
status: idea
size: S
origin: session 2026-10-21
created: 2026-10-21
milestone: M2 Window MVP
---

# Window design sketch: the list view, the states and the voice's place, before the shell is built

What: a static sketch of the window (plain HTML and CSS under `crates/mrgl-app/ui/`, no IPC yet): the
flavor tabs or sidebar, the addon list with name, version, source badge and state colors (managed,
unmanaged, linked, update available, guess only), the search field, the "update all" button, and where the
voice's two lines live (status bar, toasts). Light and dark from the start.
Why: 009 and 010 build on one agreed layout instead of inventing it in the IPC task; the states and the
voice placement are decided once.
Done when: the sketch renders in the Tauri window with made-up data, both themes, and 009 can replace the
made-up data with the core's answers without changing the layout.
