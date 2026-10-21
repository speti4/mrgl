---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 009 — Tauri shell with the flavor list

## Context

On the layout of the design sketch (017), the window finds the game folders (a chosen root,
remembered) and fills the list per flavor with each addon's name, version, source badge and state (managed,
unmanaged, linked, update available) from the core, with search. The IPC data module implements the
contract of the sketch's `mock-data.js` behind `window.Data` and replaces it in `index.html` (the mock stays
for a browser look); the core's `Message` gains a `kind` field so the window picks the voice placement; a
resolve through IPC reuses the source's `Pick` (003); the flavor header names the version's source (019);
the Forever column binds to the core's Forever status (011), a row names its missing dependencies (029),
and "copy diagnostics" puts the doctor report (024) on the clipboard. The daily overview the CurseForge
app gives today, without it.
Done when: the four flavors of a real install show their addons with the right states, offline.

Decided 2026-10-22: first of the chain 009 → 034 on one branch; the "update available" state comes from
the sidecar and the cached release list only (no network on the list), the update actions are 010.

## Docs that move

- `docs/development.md` ("The UI"): the IPC commands, the data module and how the mock stays for a
  browser look; `README.md` ("What it will do") if the window's state changes there.

## Not in this change

- Update all and update one with progress (010); the icon and the release (012); the window preferences
  (034, next in the chain); any new core feature (the window reads what the core has).

## Steps

1. [ ] Tauri commands in `crates/mrgl-app/src/commands.rs` (or the layout's name): the game root
       (discovered or chosen, remembered), the flavors with counts, the addon rows per flavor from the
       scan and the sidecar, the doctor report as text
2. [ ] `ui/data.js` behind `window.Data` on the mock's contract, wired in `index.html`; the mock stays
       for a browser look; the core's `Message` gains `kind`, the voice placement follows it
3. [ ] Rows: name, version, source badge, state, the Forever column (011), missing dependencies (029);
       the flavor header names the version's source (019); search; "copy diagnostics" (024)
4. [ ] Tests: the Rust side on fixture flavors; the Node tests of the pure modules; the window run once
       on the real install (the only session allowed to run the window; close it when done)
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
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
