---
size: S
origin: done/017-window-design-sketch-before-the-shell.md
created: 2026-10-21
started: 2026-10-22
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

1. [ ] The preference read and write in the core's home module (`config.json`: theme, serious), with the
       precedence flag > variable > file documented
2. [ ] Tauri commands to read and write them; the window's toggles call them; `localStorage` no longer
       holds them (a one-time migration of an existing value is fine)
3. [ ] Tests: the precedence in the core; the window's state after a cleared web view (a Node test of the
       pure part, the rest in Verification)
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 009 → 034

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, at close)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
