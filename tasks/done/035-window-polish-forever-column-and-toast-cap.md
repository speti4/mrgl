---
size: S
origin: done/017-window-design-sketch-before-the-shell.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M2 Window MVP
closed: 2026-10-21
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

1. [x] The "any note in this flavor" helper and the column toggle in the view; a Node test
2. [x] The toast stack cap in the toast helper (brief toasts drop oldest first, sticky ones stay); a Node test
3. [x] A look at the window in both themes; every check green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `State.hasForeverNotes(addons)` in `ui/state.js` (a row carries a Forever note when its
  `forever` slot is filled: the build verdict, with or without the curated text); the view toggles
  `without-forever` on the flavor section, and `style.css` drops the column from the grid and hides the
  header span (`forever-head`) and the cells under that class. `Voice.TOAST_CAP` (3) and
  `Voice.pushedOut(stack)` in `ui/voice.js`: the stack oldest first, the newest last; brief toasts are pushed
  out oldest first until the cap holds, a sticky one never, the newest never. The view marks a sticky toast
  with the `sticky` class and removes what `pushedOut` names after every append. Eight Node tests joined
  (39 in all): the helper over mixed rows, the cap, a sticky toast kept, the newest kept over a stack of
  sticky ones, and the ten-message burst; `mock-data.test.js` asserts that only the Forever data carries
  Forever notes. No new module file, so `docs/development.md` is unchanged.

## Deviations from the plan

- The newest toast is never pushed out either, so a stack of three sticky errors still shows the success
  that follows them; the stack then runs over the cap until an error is dismissed. The alternative, dropping
  the message the reader has not seen yet, would make the cap hide news.

## Verification

- 2026-10-21 — `node --test` over `tests/ui/*.test.js`: 39 passed (31 before); `cargo fmt --check`,
  `cargo clippy --all-targets --locked -- -D warnings`, `cargo test -p mrgl-app --locked` (`ui_smoke` runs
  the same 39), `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check`,
  all green in the worktree.
- 2026-10-21 — the window (`target/debug/mrgl-app.exe` of the worktree) driven by a script with clicks and a
  screenshot per step, in the dark theme and in the light one after the toggle: Forever opened with the
  Forever column (header and the yes / no / unknown dots with the curated notes); retail showed five columns
  and no Forever header, the remaining columns taking the width; era's "Update all" put the sticky error
  toast at the bottom right; then classic's "Update all" six times in a row (the first run and five
  immediate repeats, a warning each): the stack held three toasts, the sticky error on top and the two
  newest warnings under it, in both themes; after the warnings timed out the error alone remained until
  dismissed. The window was closed after the look.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
