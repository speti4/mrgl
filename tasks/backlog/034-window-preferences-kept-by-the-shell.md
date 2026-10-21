---
status: idea
size: S
origin: done/017-window-design-sketch-before-the-shell.md
created: 2026-10-21
milestone: M2 Window MVP
---

# Window preferences kept by the shell: theme and the serious switch out of `localStorage`

What: the theme choice (auto, light, dark) and the serious switch of the window move from the sketch's
`localStorage` into a setting the shell keeps under the mrgl home folder, so `MRGL_SERIOUS=1`, `--serious`
and the window's switch agree and a reset of the web view does not lose them.
Why: a preference in the web view's storage is invisible to the CLI and dies with the web view's data.
Done when: the switch in the window and the environment variable report the same state, and the setting
survives a cleared web view.
