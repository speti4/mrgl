---
size: S
origin: done/017-window-design-sketch-before-the-shell.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M2 Window MVP
---

# 035 — Window polish: hide the Forever note column without notes, cap the toast stack

## Context

Two things the sketch (017) left for later: an empty Forever note column header is noise on three of the
four flavors, and a burst of updates must not bury the list under toasts. In the sketch's view code under
`crates/mrgl-app/ui/`: the Forever note column (header and cells) is hidden when no row of the shown flavor
carries a note, and the toast stack keeps only the newest few (a sticky error or fatal stays until
dismissed and is never pushed out by a brief toast). Both as pure helpers with Node tests where the logic
lives, the view calling them.
Done when: the retail made-up data shows no Forever column, the Forever data shows it, and ten quick
messages leave the capped number of toasts with the sticky error on top.

Decided 2026-10-21: the cap is a constant in the toast helper (three visible), not a preference.

## Docs that move

- `docs/development.md` ("The UI") only if a new module file joins; otherwise none.

## Not in this change

- IPC (009), the preferences (034), any change to the data contract of `mock-data.js`.

## Steps

1. [ ] The "any note in this flavor" helper and the column toggle in the view; a Node test
2. [ ] The toast stack cap in the toast helper (brief toasts drop oldest first, sticky ones stay); a Node test
3. [ ] A look at the window in both themes; every check green; close

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
