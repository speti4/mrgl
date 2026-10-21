---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M2 Window MVP
---

# 017 — Window design sketch: the list view, the states and the voice's place, before the shell is built

## Context

009 and 010 build on one agreed layout instead of inventing it in the IPC task; the states and the voice
placement are decided once. A static sketch of the window under `crates/mrgl-app/ui/` (plain HTML, CSS and
JavaScript, no IPC): the flavor tabs or sidebar, the addon list with name, version, source badge and the
state colors (managed, unmanaged, linked, update available, guess only), the search field, the "update all"
button, and where the voice's two lines live (status bar, toasts). Light and dark from the start, made-up
data in one JavaScript module that 009 replaces with the core's answers without changing the layout.
Done when: the sketch renders in the Tauri window with made-up data in both themes, and 009 can swap the
made-up data for the core's answers without touching the layout.

Decided 2026-10-21: the sketch is the one wave-1 session that runs the window; the pure modules (state
mapping, search filter, the voice placement) get Node tests under `tests/ui/`.

## Docs that move

- `docs/development.md` ("The UI"): the layout of `ui/` (the data module, the view modules, the themes).
- `docs/nerglish.md` if the voice's place in the window (status bar, toasts) adds a rule.

## Not in this change

- IPC and the Tauri commands in `crates/mrgl-app/src/` (009), update actions with progress (010).
- The murloc icon (012).
- Real data from the core: every row is made up.

## Steps

1. [ ] The data module with made-up flavors and addons covering every state
2. [ ] The layout: flavor navigation, the list with badges and state colors, search, "update all"
3. [ ] The voice's two lines: status bar and toasts; light and dark themes
4. [ ] Node tests for the pure modules; `cargo test -p mrgl-app` runs them through `ui_smoke`
5. [ ] A look at the window (`cargo run -p mrgl-app`) in both themes; docs; every check green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, when the work is done)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the stale index is the expected finding on a task branch)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
