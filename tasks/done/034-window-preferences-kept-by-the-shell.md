---
size: S
origin: done/017-window-design-sketch-before-the-shell.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 034 — Window preferences kept by the shell: theme and the serious switch out of `localStorage`

## Context

The theme choice (auto, light, dark) and the serious switch of the window move from the sketch's
`localStorage` into a setting the shell keeps under the mrgl home folder, so `MRGL_SERIOUS=1`, `--serious`
and the window's switch agree and a reset of the web view does not lose them. A preference in the web
view's storage is invisible to the CLI and dies with the web view's data.
Done when: the switch in the window and the environment variable report the same state, and the setting
survives a cleared web view.

Decided 2026-10-22: second of the chain 009 → 034; the setting lives in the same `config.json` under the
home folder that remembers the game root (D-003), read by the CLI's voice as a third source after the
flag and the variable.

## Docs that move

- `docs/development.md` ("The UI", the home folder's files): the preference keys and their precedence.

## Not in this change

- Any other preference (window size, last flavor); a settings page.

## Steps

1. [x] The preference read and write in the core's home module (`config.json`: theme, serious), with the
       precedence flag > variable > file documented
2. [x] Tauri commands to read and write them; the window's toggles call them; `localStorage` no longer
       holds them (a one-time migration of an existing value is fine)
3. [x] Tests: the precedence in the core; the window's state after a cleared web view (a Node test of the
       pure part, the rest in Verification)
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 009 → 034
- 2026-10-22 — core: `home::Preferences` with `read_preferences_at`, `update_preferences_at` (a key not
  given stays, an unknown theme is refused) and `read_preferences`; `write_config_at` now sets or removes
  the keys `Config` owns and keeps every other key of the file, so `mrgl root` keeps the preferences;
  `voice::serious_from` (flag, then a non-empty `MRGL_SERIOUS`, then the file, read only when needed) with
  `SeriousSource`, and `Voice::from_env` reads the file as the third source, every call site unchanged
- 2026-10-22 — window: the commands `preferences` and `set_preferences`; `ui/preferences.js` (what the
  view draws from an answer, the switch locked when `MRGL_SERIOUS` decides it, the one-time carry-over of
  the sketch's `localStorage` values); `main.js` reads the preferences at start, writes each toggle's
  change, and removes the old `localStorage` keys
- 2026-10-22 — tests: the precedence (`voice.rs`), the preferences next to the root and the odd values
  (`home.rs`), the answer of the window from the file and the variable (`commands.rs`), the CLI end to end
  with a `config.json` that says serious and with `MRGL_SERIOUS=0` over it, `tests/ui/preferences.test.js`
  and the contract's new calls in the data and mock tests; `cargo test` green

## Deviations from the plan

- `MRGL_SERIOUS=0` is an explicit "not serious" that wins over the file, so the variable decides in both
  directions; an empty variable counts as not set. The window's switch is disabled with the title "set by
  MRGL_SERIOUS" while the variable decides.
- `docs/nerglish.md` names the third source next to the flag and the variable.

## Verification

- 2026-10-22 — the window on a made-up home folder (`MRGL_HOME`): at the first start the web view still
  held the sketch's `localStorage` values (dark, not serious); the window carried both into `config.json`
  (`"theme": "dark"`, `"serious": false`) and removed the old keys. Turning the switch on and the theme to
  auto wrote `"serious": true`, `"theme": "auto"`; after `localStorage.clear()` and a reload the window drew
  auto (dark) with the switch on and the status bar without its murloc line, and `mrgl --version` on the
  same home folder printed the plain line alone. Started again with `MRGL_SERIOUS=0`: the switch showed off,
  disabled, titled "set by MRGL_SERIOUS", the status bar spoke murloc again, and `mrgl --version` with the
  same variable printed both lines: the window and the command line report the same state.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
