---
size: M
origin: done/014-addon-sets-with-enable-and-disable.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 074 — The window's toolbox: a set switch and a SavedVariables view

## Context

Per flavor, a set picker that applies a set to every character or to one (`sets::apply_set` with a
`Target`), and a SavedVariables view with sizes, orphans and a cleanup button (`savedvars`), both refused
while the client runs with the voice's line. The toolbox commands of M3 exist in the CLI only; the window is
where most users will look for them.
Done when: on a fixture flavor the window applies a set and moves the orphans into the dated zip, and both
are refused with the stand-in client running.

Decided 2026-10-22: one task on its own branch in wave 6, built on the core's toolbox as it is today
(`sets::apply_set`, `sets::find_character`, `savedvars`); the set editing and the undo of 073 land in the
command line first and join the window later.

## Docs that move

- `docs/development.md`: the UI section (the toolbox panel, its pure modules and their tests) and the IPC
  commands (the set and SavedVariables commands).
- `README.md`: the window's feature list, where the toolbox is named.

## Not in this change

- Saving a set from the window; the set undo and the set editing (073); the `WTF` backup and restore in the
  window; a new online request.

## Steps

1. [x] IPC commands in `mrgl-app/src/commands.rs` over plain functions of a new `src/toolbox.rs`: the sets of a
       flavor with their addons, the characters, `apply_set` to every character or one, the SavedVariables
       listing (sizes, orphans, client files) and the cleanup into the dated zip; every write runs the
       running-client guard (023) first and answers the refusal through the voice
2. [x] `ui/`: a toolbox panel per flavor (the set picker with the target, the SavedVariables table with the
       cleanup button), its pure module(s) with Node tests under `tests/ui/`, the mock data for the sketch
3. [x] Tests in `mrgl-app/tests/`: on a fixture flavor the window applies a set (every character and one) and
       moves the orphans into the dated zip; both refused with the stand-in client running
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — `src/toolbox.rs`: `load`, `apply` and `clean` take the game root, the flavor id, the mrgl home
  and the voice. `load` reads the sets (`sets::read_sets_at`, each addon with whether its folder is under
  `Interface\AddOns`), the characters (`sets::characters`), the SavedVariables listing
  (`savedvars::list_saved_variables`, sizes as `zip_entry::size_text` writes them) and the running client
  (`client::client_running`, a warning with the guard's line). `apply` runs
  `client::ensure_client_not_running` first, pins the window's pick through `sets::find_character`, then
  calls `sets::apply_set` with its `Target`; `clean` runs the guard, then `savedvars::clean_orphans` into
  `backup::backups_dir_at(home, flavor)`. The lines are those of `mrgl set apply` and `mrgl savedvars`; the
  account folder's name stays out of the answer, as out of the command line's plain lines. `sets.rs` and
  the rest of the core are unchanged.
- 2026-10-22 — `commands.rs`: `toolbox`, `apply_set` and `clean_saved_variables`, each on a blocking thread.
  One toolbox write at a time (`toolbox::ToolboxRun`, managed state); the cleanup is refused with a warning
  while an update runs, and an update run waits for a toolbox write, since an install swaps addon folders
  and the orphan rule reads them.
- 2026-10-22 — UI: a tab row under the flavor header ("Addons", "Toolbox"); the toolbox tab holds an "Addon
  sets" card (set picker, "Apply to" with every character or one by name and realm, the set's addons as
  chips, the apply button, what the last apply changed) and a "SavedVariables" card (summary, "orphans
  only" filter, the table, the cleanup button naming the orphans). A warning line names a running client;
  the buttons stay enabled and the core's guard answers. The pure part is `ui/toolbox.js`
  (`tests/ui/toolbox.test.js`); `data.js` and the contract in `mock-data.js` carry `Data.toolbox`,
  `Data.applySet` and `Data.cleanSavedVariables`; the mock's retail client "runs", so the sketch shows the
  refusal. Theme tokens of `style.css` only, checked in the light and the dark theme.
- 2026-10-22 — `tests/toolbox.rs`: a made-up Forever flavor with the addon-sets fixture's `WTF` tree (two
  characters on two realms), three addon folders, five SavedVariables entries (two orphans, one client file)
  and two sets; the listing, an apply to every character and to one (the other character's `AddOns.txt`
  byte for byte), the core's errors for an unknown character and set, the cleanup into
  `SavedVariables-<date>.zip` with the orphans gone and the rest kept, no home folder, the answer's keys
  against the contract, and both writes refused while the stand-in client of 063 runs, with `WTF` unchanged
  and no zip written.

## Deviations from the plan

- The cleanup refuses while an update runs, and an update run refuses while a toolbox write goes: an install
  swaps addon folders, and a folder away for a moment would make its SavedVariables an orphan.
- The window's pick is pinned through `sets::find_character` after the guard, so the refusal of a running
  client comes before any `WTF` read, as the task asks; `sets::apply_set` resolves the same character again.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green, among them the 8 of `crates/mrgl-app/tests/toolbox.rs`
  (the stand-in client test included) and the 71 Node tests of `tests/ui/` through `ui_smoke.rs`.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo +1.90 check --all-targets --locked`, `python -I scripts/changelog.py --check`: clean.
- 2026-10-22 — The window (`target/debug/mrgl-app.exe`) against a made-up flavor in a temporary folder
  (`MRGL_ROOT` and `MRGL_HOME` set, the addon-sets fixture's `WTF`, two sets saved with `mrgl set save
  --character`), driven over the web view's debug port: the toolbox listed 2 sets, the 2 characters in the target picker after "every character", and
  `3 SavedVariables files of _forever_, 88 B in all; 2 orphans (50 B) have no addon folder: …`; "Apply raid"
  to one character answered `applied set raid to Fixture Realm/Mrgltest of _forever_: 2 addon lines changed`,
  "Apply solo" to every character `applied set solo to 2 characters of _forever_: 5 addon lines changed`;
  the cleanup `moved 2 orphans of _forever_ (3 files, 50 B) into …\backups\wowf\SavedVariables-<date>.zip`,
  and the summary turned to `no orphans`. With the stand-in client (a copy of the system's `ping.exe` in the
  flavor folder) running, the panel named it and both the apply and the cleanup answered `the client is
  running in _forever_ (client-stand-in.exe, pid …): close the game first, nothing under WTF is written while
  it runs`, the orphan left in place.
- 2026-10-22 — The sketch (`ui/index.html` with `mock-data.js`, served locally) shows the toolbox tab in both
  themes; retail's made-up client refuses both writes.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
