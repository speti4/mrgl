---
size: M
origin: done/014-addon-sets-with-enable-and-disable.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 073 — Addon sets: undo an apply, edit a stored set, `--account`, and uninstalled addons on save

## Context

`set apply` keeps the `AddOns.txt` files it replaced so `set undo` puts them back; `set enable|disable|drop
<set> <addon>` edits a stored set without the files; `--account` picks a character when one realm and name
sit under two account folders (027); `set save` warns about addons that are no longer installed. A wrong
apply has no way back today, and a set can only change by saving it again from the files.
Done when: a fixture apply round-trips through undo byte for byte, an edit changes only the named addon, and
the two-account case resolves with `--account`.

Decided 2026-10-22: second of the chain 062 → 073 on one branch, in wave 6. `set save` keeps an addon that
is no longer installed and warns about it (a set is the user's data; `set apply` already writes such an
addon's line with a warning). `sets::apply_set` keeps its signature and its result: the undo record is
written by the apply itself under the flavor's folder of the mrgl home, so a caller of today (the window's
toolbox, 074) is unchanged; new behavior is a new function.

## Docs that move

- `docs/development.md`: the addon sets paragraph (the undo record and its folder, the edit verbs,
  `--account`, the save warning).
- `README.md`: the `set` commands, where they are listed.

## Not in this change

- The window's set switch (074); a set shared between flavors; an undo of anything but the last apply.

## Steps

1. [x] `sets.rs`: `apply_set` keeps the replaced `AddOns.txt` files (and the names of the files it created)
       under `<home>\<flavor-id>\set-undo\`, the last apply only; `undo_apply` puts them back byte for byte
       and removes the files the apply created, refused while the client runs (023)
2. [x] `sets.rs`: `enable`, `disable` and `drop` of one addon in a stored set, in `sets.json` only; the
       character lookup takes an account (`--account`) when one realm and name sit under two accounts
3. [x] `set save` warns about addons of the set that are not installed and keeps them
4. [x] CLI: `set undo`, `set enable|disable|drop <set> <addon>`, `--account` on `save` and `apply`, each
       with `--json` (`set-undo`, `set-edit`) and the voice
5. [x] Tests in `tests/sets.rs` and `tests/cli.rs`: the apply round-trips through undo byte for byte, an edit
       changes only the named addon, the two-account case resolves with `--account`, the save warning
6. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 062 → 073
- 2026-10-22 — `sets.rs`, the undo: `apply_set` keeps its signature and its `Applied`; it delegates to
  `apply_set_in_account`, which writes the undo record of the files that change into `set-undo.new\`
  next to `sets.json` (`undo.json` with `format` `mrgl-set-undo`, `schema` 1, the flavor, the set,
  `applied_at` and per file the account, realm and character names, `<n>-before.txt` with the replaced
  bytes, absent for a created file, and `<n>-after.txt` with the written bytes), sets the record before
  it aside and moves the new one in, all before the first `AddOns.txt` write; after the writes the old
  record goes, and a failed apply whose files are back puts it back. An apply that changes nothing keeps
  the record before it. `undo_apply` reads the record and every copy, refuses a record of another flavor
  or schema or one that names a path (`SetUndoUnreadable`), runs the guard of task 023, writes the
  replaced files back and removes the created ones with the written ones put back on a failure
  (`SetUndoFailed`), deletes the record (`NothingToUndo` on a second undo), and warns about a file that
  changed since the apply or a character folder that is gone.
- 2026-10-22 — `sets.rs`, the edits: `edit_set` with `SetEdit` (`enable`, `disable`, `drop`) changes one
  addon of a stored set in `sets.json` only, matched ignoring ASCII case with the set's spelling kept; an
  enable or a disable of an addon the set lacks adds it, a drop of one it lacks is `AddonNotInSet`, a name
  no `AddOns.txt` line can carry is `AddonName`, an edit to the same state writes nothing, and an edit
  that changes the set moves its `saved_at`. `Edited` says whether the addon is installed.
- 2026-10-22 — `sets.rs`, the account: `targets_in_account`, `find_character_in_account`,
  `save_set_in_account` and `apply_set_in_account` narrow the characters to one account folder, ignoring
  case (`AccountNotFound` lists the accounts with characters); `targets`, `find_character`, `save_set` and
  `apply_set` keep their signatures and pass no account. The two-account error's hint reads "name one with
  --account".
- 2026-10-22 — `sets.rs`, the save: `Saved::not_installed` names the addons of the saved set without a
  folder under `Interface\AddOns`; they stay in the set.
- 2026-10-22 — CLI: `set undo` (data lines per character, the warnings, the summary; `--json` kind
  `set-undo` with `restored` and `removed`), `set enable|disable|drop <set> <addon>` (a warning for an
  addon that is not installed, the summary ending "no AddOns.txt was touched", or "… already, nothing
  changed"; `--json` kind `set-edit` with `action`, `before`, `after`, `changed`, `installed` and the
  set), `--account` on `save` and `apply` (requires `--character`), and the save warning ("set <name>
  names addons that are not installed in <flavor folder>, kept in the set all the same: …";
  `not_installed` in `set-save`). Every line goes through the reporter's voice.
- 2026-10-22 — tests: `tests/sets.rs` `an_apply_round_trips_through_undo_byte_for_byte` (three
  characters, one without `AddOns.txt`: the `WTF` tree after the undo equals the one before byte for byte,
  the created file gone; a no-op apply keeps the record; a second undo is `NothingToUndo`; of two applies
  only the last is undone), `an_undo_warns_about_a_file_changed_since_the_apply_and_puts_it_back_all_the_same`,
  `an_undo_record_that_names_a_path_is_refused`, `an_undo_is_refused_while_the_client_runs` (Windows, the
  stand-in client of task 063), `an_edit_changes_only_the_named_addon_in_the_sets_file`,
  `one_realm_and_name_under_two_accounts_resolve_with_the_account`,
  `a_save_keeps_an_addon_that_is_not_installed_and_names_it`; the failed-write test also checks that no
  undo record is left behind. `tests/cli.rs` `set_undo_puts_the_last_apply_back_byte_for_byte`,
  `set_undo_is_refused_while_the_client_runs` (Windows), `set_enable_disable_and_drop_edit_one_addon_of_a_stored_set`,
  `set_account_picks_a_character_under_two_accounts`; the save lines and the `set-save` document of the
  existing tests carry the new warning and `not_installed`.
- 2026-10-22 — docs: `docs/development.md` (the CLI line, the `--json` kinds, `--account`, a paragraph on
  the undo record and its folder, the edit verbs and the save warning, the home folder layout), `README.md`
  (two `set` lines in the command list, the addon sets paragraph), `CLAUDE.md` (the `set-undo` folder
  under "Data outside the repo").

## Deviations from the plan

- `--account` is passed through new `_in_account` functions instead of a field or a variant of `Target`,
  so `Target`, `find_character`, `save_set` and `apply_set` stay as the window's set switch (074) uses
  them; on the command line `--account` requires `--character`.
- An apply that changes no file leaves the record of the apply before it in place, so a repeated apply
  does not take away the undo of the one that changed something.
- `set edit` is not one verb: the edits are `set enable`, `set disable` and `set drop`, with one
  `set-edit` JSON kind; an edit of an addon that is not installed warns too, so a typo shows.
- `CLAUDE.md` names the `set-undo` folder under "Data outside the repo".

## Verification

- 2026-10-22 — `cargo test --locked -p mrgl-core --test sets`: 19 passed, among them the seven tests of
  this task; `cargo test --locked -p mrgl-cli --test cli set_`: 12 passed, among them the four of this
  task.
- 2026-10-22 — a manual run of the debug `mrgl.exe` on a scratch copy of the `addon-sets` fixture with
  `MRGL_HOME` in a scratch folder: `set save raid --character Mrgltest` warned about the four addons that
  are not installed, `set apply raid` changed three lines of `Grrlwave`, `set undo` put that one file back
  with the murloc line on top, a second `set undo` exited 1 with "nothing to undo in _forever_: …", and
  `set drop raid QuestArrows` changed `sets.json` only.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green), `cargo +1.90 check --all-targets --locked` and `python -I
  scripts/changelog.py --check` green before the close; the changelog regenerated with the close.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
