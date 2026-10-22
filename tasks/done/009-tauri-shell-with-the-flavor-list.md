---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
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

1. [x] Tauri commands in `crates/mrgl-app/src/commands.rs` (or the layout's name): the game root
       (discovered or chosen, remembered), the flavors with counts, the addon rows per flavor from the
       scan and the sidecar, the doctor report as text
2. [x] `ui/data.js` behind `window.Data` on the mock's contract, wired in `index.html`; the mock stays
       for a browser look; the core's `Message` gains `kind`, the voice placement follows it
3. [x] Rows: name, version, source badge, state, the Forever column (011), missing dependencies (029);
       the flavor header names the version's source (019); search; "copy diagnostics" (024)
4. [x] Tests: the Rust side on fixture flavors; the Node tests of the pure modules; the window run once
       on the real install (the only session allowed to run the window; close it when done)
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 009 → 034
- 2026-10-22 — `src/list.rs`: the flavor list as plain functions over an explicit game root and mrgl home
  folder (`load`, `flavor_view`, `rows`, `cached_update`, `resolve`); `src/commands.rs`: the commands
  `app_version`, `flavors`, `choose_root`, `diagnostics`, `resolve`, `update_all`, the disk and network
  work on a blocking thread; the IPC `Message` carries `kind`. `ui/data.js` answers the contract through
  them; `index.html` loads it instead of the mock
- 2026-10-22 — the rows: the scan's title without color escapes and its junction flag, the sidecar's
  version, source, pin and ignore, a TOC hint for an unmanaged addon (Wago, WoWInterface, CurseForge, the
  order of D-002), the dependency check's `missing`, the Forever status on every row of Forever; "update
  available" when the cached release list's current release on the entry's channel is not the installed
  version, read through an `Http` that answers "not modified" to every request, so the list never opens a
  socket. The header names the build's file (`.build.info` or the client exe)
- 2026-10-22 — tests: `tests/flavor_list.rs` (11 tests on a made-up game root with two flavors, sidecars
  and the release cache filled by the GitHub source from the fixture answers: every state's facts, the
  contract's keys, no cache, no home folder, no root, a discovered root, a chosen flavor folder remembered,
  a folder refused, a resolve's `Pick`), 4 unit tests of the `Message`, `tests/ui/data.test.js` (the
  commands and their arguments) and the mock's test on the grown contract; `cargo test` green
- 2026-10-22 — the window run on the real install and on a made-up game folder (Verification)

## Deviations from the plan

- The core has no `Message` type: the window's IPC `Message` (`crates/mrgl-app/src/commands.rs`) gains
  `kind`, the name `Kind::as_str` gives; `mrgl_core::voice` is unchanged.
- The contract of `mock-data.js` grew with the shell: `Data.chooseRoot()`, `Data.diagnostics()`,
  `Data.resolve(flavorId, folder)`, `root_origin` on the answer, `version_source` and `version_file` on a
  flavor, `missing` on a row, and the full Forever status (`kind`, `verified`, `source`, `stale` next to
  `build` and `note`). The mock answers every call, so the browser look stays complete.
- "Update available" means the cached current release is not the installed version (the release the
  source calls current), not a version comparison; the update check of task 006 is not on `main` yet.
- The browser look: `index.html` loads `data.js` only; without the Tauri global `data.js` writes the
  mock's script tag (`document.write`, so the mock runs before the deferred `main.js`).
- "Update all" answers that the update actions are task 010, as a warning in the voice, until 010 lands.
- The clipboard: `navigator.clipboard.writeText` with a selection copy as the fallback; no clipboard
  plugin, no new crate.

## Verification

- 2026-10-22 — the window on the real install, offline as to the list (the cache-only source makes no
  request): the four flavors in the sidebar in the order retail, classic, era, forever with their addon
  counts, opened on Forever. Retail and classic show "build … from .build.info" and "managed by the
  CurseForge app"; era shows its build read from `WowClassic.exe` (task 019); forever shows its build from
  `.build.info` and "no addons in _forever_" (no `Interface\AddOns` there yet). No flavor has a sidecar, so
  every row is "guess only" (a dashed Wago, WoWInterface or CurseForge badge from the TOC hint) or
  "unmanaged", the right states for folders mrgl does not manage yet. The search "wago big" narrowed retail
  to the matching rows; one retail row names a missing dependency (a `Blizzard_*` addon of the client
  itself, a follow-up). Both themes, light and dark, on every flavor; the status bar's banner names the
  flavor and addon counts. Nothing was written under the game folder.
- 2026-10-22 — the window on a made-up game folder and home (`MRGL_ROOT`, `MRGL_HOME`, `MRGL_HTTP_STUB`):
  retail shows a GitHub row "update available 1.2.2 → 1.2.3" (the sidebar's "1↑", "Update all (1)"), a
  Wago guess, an unmanaged row with "missing: LibGone"; forever shows the Forever column with a linked row
  (`link:` source), a managed GitHub row with Forever build "yes" from the cached `release.json`, a zip row
  "pinned at 1.0, ignored" and a CurseForge guess, each with its Forever status. "Copy diagnostics" put the
  doctor's text on the clipboard with a success toast; "Update all" answered the task-010 warning as a
  toast and in the status bar; "Game folder…" opened the dialog "Choose the World of Warcraft folder" and
  a cancel changed nothing. Light and dark.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
