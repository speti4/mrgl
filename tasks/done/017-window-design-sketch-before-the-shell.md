---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M2 Window MVP
closed: 2026-10-21
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

Decided 2026-10-21, in the sketch:

- A **sidebar**, not tabs: the four flavors by their friendly names in the order retail, classic, era,
  forever, each with its addon count and its count of updates; the window opens on Forever. The sidebar has
  room for the later pages (backups, sets, doctor) and its bottom holds the preferences (theme, serious).
- **The voice's places**: every message replaces the status bar's line (a dot in the kind's color, the murloc
  line, the plain line); a finished message (success, warning, error, fatal) also pops a toast at the bottom
  right above the status bar, with the murloc line as its headline and the plain line under it. Success and
  warning toasts go away by themselves, error and fatal stay until dismissed. Banner, progress and
  nothing-to-do never toast. The toasts sit at the bottom so a sticky error never covers "Update all".
- **The row's state is derived in the view** from the facts the data carries (`linked`, `update`,
  `managed`, `guessed`), with the precedence linked, update, managed, guess, unmanaged; the data never
  names a state. A source known only from a TOC hint draws as a dashed badge.
- **The theme** is a preference with three values (auto, light, dark): auto follows
  `prefers-color-scheme`; every color is a token on `:root`, so a state looks the same in a row, a pill and
  the legend in both themes. The serious switch drops the murloc line in both places; the murloc line
  itself always arrives in the message, the view never composes one.
- **The data contract** is the header of `ui/mock-data.js`: `Data.version()`, `Data.flavors()` (root,
  flavors with addons, a banner message) and `Data.updateAll(flavorId, report)`; a message is
  `{ kind, murloc, plain }`, which means the Rust `Message` of 009 gains a `kind` field.

## Docs that move

- `docs/development.md` ("The UI"): the layout of `ui/` (the data module, the view modules, the themes).
- `docs/nerglish.md` if the voice's place in the window (status bar, toasts) adds a rule.

## Not in this change

- IPC and the Tauri commands in `crates/mrgl-app/src/` (009), update actions with progress (010).
- The murloc icon (012).
- Real data from the core: every row is made up.

## Steps

1. [x] The data module with made-up flavors and addons covering every state
2. [x] The layout: flavor navigation, the list with badges and state colors, search, "update all"
3. [x] The voice's two lines: status bar and toasts; light and dark themes
4. [x] Node tests for the pure modules; `cargo test -p mrgl-app` runs them through `ui_smoke`
5. [x] A look at the window (`cargo run -p mrgl-app`) in both themes; docs; every check green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `ui/mock-data.js` (28 made-up addons over the four flavors: every state, every source badge,
  a pinned and an ignored row, the Forever notes on every Forever row, one "update all" outcome per flavor
  so every kind of the voice shows somewhere), `state.js`, `search.js`, `theme.js`, `flavors.js`,
  `voice.js` with `placement`, the view in `main.js`, the page and the stylesheet; 31 Node tests in six
  files, run by `ui_smoke`
- 2026-10-21 — the first look at the window found the empty-state greeting drawn above the list: a
  `display` rule on the section beat the `hidden` attribute; fixed with a `[hidden]` rule that wins.
  `document.title` does not reach the Tauri window title; dropped
- 2026-10-21 — the third look found a sticky error toast in the top right covering "Update all"; the toast
  stack moved to the bottom right, above the status bar
- 2026-10-21 — `docs/development.md` ("The UI": the files and the contract) and `docs/nerglish.md` (rule 5
  and a new section "The window") changed

## Deviations from the plan

- The theme toggle has three values (auto, light, dark) instead of a light / dark switch, so a user who
  follows the system keeps following it.
- A serious switch joined the sidebar: rule 5 of `docs/nerglish.md` calls the serious setting a preference,
  and the sketch shows what both places look like without the murloc line.

## Verification

- 2026-10-21 — `node --test` over `tests/ui/*.test.js`: 31 passed; `cargo fmt --check`, `cargo clippy
  --all-targets --locked -- -D warnings`, `cargo test --locked` (the app's unit tests and `ui_smoke`, the 23
  CLI tests, the 30 core unit tests and 10 scan tests), `cargo +1.90 check --all-targets --locked` and
  `python -I scripts/changelog.py --check`, all green in the worktree.
- 2026-10-21 — the window (`target/debug/mrgl-app.exe` of the worktree) driven by a script with clicks and
  keystrokes, screenshots per step, in the dark theme (the system's) and in the light one after the toggle:
  the sidebar listed retail 8, classic 6, era 4 and forever 10 addons with their update counts and opened on
  Forever; the Forever list showed the five states (BigWigs update available, Questie managed, Details
  guess only with a dashed CurseForge badge, My Auras linked, Raid Logger unmanaged) and the six badges
  across the flavors, the Forever notes with their yes / no / unknown dots and the curated text, the
  pinned and the ignored row in classic, the warning line of era; the search `quest` kept Questie alone and
  `zzz` showed the no-match line; "Update all" on Forever put the progress line in the status bar with the
  rows' progress bars, then the success toast and the flipped versions; classic gave the warning toast with
  the pinned and the ignored counts; era gave the sticky error toast and the red dot in the status bar;
  retail gave "nothing to update" in the status bar alone; the row menu opened with its four disabled
  actions; the serious switch left the plain line alone in both places; the theme and the switch were
  remembered across runs. The window was closed after each look.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
