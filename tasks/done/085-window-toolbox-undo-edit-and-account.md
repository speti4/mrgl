---
size: M
origin: done/074-window-toolbox-sets-and-savedvariables.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 085 — The window's toolbox undoes an apply, edits a set and names the account

## Context

The toolbox tab (074) offers the undo of the last apply (`sets::undo_apply`), the enable, disable and drop of
one addon in a stored set (`sets::edit_set`), and passes an account through `sets::apply_set_in_account` when
one realm and name sit under two accounts (073), each refused while the client runs as the apply is.
Why: 073 landed the undo, the edit and `--account` on the command line in the same wave as the window's set
switch, so the window applies sets but cannot take one back or change one.
Done when: on the fixture flavor the window undoes an apply byte for byte, an edit changes only the named
addon in `sets.json`, and the two-account case applies to the chosen account.

Decided 2026-10-22: runs after 086 on the same branch, on top of the shared summary lines.

## Docs that move

- `docs/development.md`: the UI section (the toolbox's undo, edit and account controls, `toolbox.js`) and the
  IPC commands (the new toolbox commands and their answers)
- `README.md`: the window's feature list and the usage line of the window, where the toolbox is described

## Not in this change

- Saving a set from the window; the `WTF` backup and restore in the window; a new core function (073 has them)

## Steps

1. [x] `crates/mrgl-app/src/toolbox.rs` and `commands.rs`: the undo (`sets::undo_apply`), the edit
       (`sets::edit_set`: enable, disable, drop) and the apply with an account (`sets::apply_set_in_account`),
       each behind the running-client guard and the one-write-at-a-time rule of 074, every message through
       the voice (the edit behind the one-write rule only, see *Deviations*)
2. [x] `ui/`: the undo button with what it restores, the edit controls on the set's chips, the account pick
       when a character sits under two accounts; the pure part in `ui/toolbox.js` with Node tests, the
       mock data with the two-account case
3. [x] Tests in `crates/mrgl-app/tests/toolbox.rs`: the undo restores byte for byte, an edit changes only the
       named addon in `sets.json`, the two-account case applies to the chosen account, each refused with the
       stand-in client running (the edit goes through, see *Deviations*)
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — core, as task 086 did for the apply: `sets::Undone::summary`, `sets::Edited::summary` and
  `sets::Edited::not_installed` (the lines of `mrgl set undo` and `mrgl set enable|disable|drop`),
  `sets::AddonSet::counted` (`3 addons (2 enabled, 1 disabled)`), and `sets::undo_record`, the last
  apply's record read as `undo_apply` reads it, without a write, so the window names what an undo puts
  back before it is pressed. The CLI prints the core's lines; its `count_addons_txt` and the copies of
  the two summaries are gone. The undo summary's kind follows the undo's own warnings (a flavor warning
  alone no longer turns it into a warning).
- 2026-10-22 — `src/toolbox.rs`: `CharacterView` gains `account`, set only when another account has a
  character of the same realm and name (compared ignoring case), so the account folder's name stays out
  of every other answer; `ToolboxView` gains `undo` (`UndoView`: the set, `applied_at`, `restore` and
  `remove`), `ToolboxAnswer` gains `undone`. `apply` takes a `Pick` (`character`, `realm`, `account`) and
  goes through `sets::find_character_in_account` and `sets::apply_set_in_account`; `undo` runs the guard,
  then `sets::undo_apply`; `edit` takes `enable`, `disable` or `drop` and calls `sets::edit_set`. Every
  line is the core's, through the voice. The busy lines name the undo and the edit too.
- 2026-10-22 — `commands.rs`: `apply_set` takes `account`; `undo_set` and `edit_set` are new; the three
  and the cleanup share one `toolbox_write` helper over `ToolboxRun` (one toolbox write at a time).
  `lib.rs` registers the two new commands.
- 2026-10-22 — UI: the "Addon sets" card gains an "Undo <set>" button next to the apply, with the line
  under it that says which `AddOns.txt` files the undo puts back and which it removes; each chip is a
  toggle between enabled and disabled in the set with a `×` that drops the addon, and a hint line under
  the chips; the target picker names the account when two accounts have the realm and the name; the lines
  under the card show what the last apply changed or the last undo put back. `ui/toolbox.js` gains
  `characterName`, `undoNote`, `undoButton`, `undoneLines`, `chipActions` and `BUSY`, and the account in
  `targetOptions` and `targetOf`; `data.js` gains `Data.undoSet` and `Data.editSet` and passes the
  account; `mock-data.js` gains a second made-up account with a twin of one character, a made-up undo
  record and edit, and the contract in its header. Theme tokens of `style.css` only; the line-through of a
  disabled chip moved from the chip to its name so the `×` stays plain.
- 2026-10-22 — tests: `tests/toolbox.rs` `an_undo_puts_the_apply_back_byte_for_byte` (one file put back,
  one the apply created removed, the `WTF` tree equal byte for byte, a second undo the core's "nothing to
  undo"), `an_edit_changes_only_the_named_addon_in_the_sets_file` (only `RaidFrames` changes in
  `sets.json`, the other set equal, an edit to the same state writes nothing, the drop, the add with its
  warning, an unknown action and an addon not in the set refused, `WTF` untouched),
  `one_realm_and_name_under_two_accounts_apply_to_the_chosen_account` (the account only on the twins, the
  apply without it the core's ambiguity error, with it the twin's file alone changed),
  `the_wtf_writes_are_refused_while_the_stand_in_client_runs` (the apply, the apply with an account, the
  undo and the cleanup refused with `WTF` unchanged, the edit goes through), the contract keys
  (`undone`, `undo`, `account`) and the no-home answers of the undo and the edit. `tests/sets.rs` checks
  `sets::undo_record` before and after an apply. Node: five new tests in `tests/ui/toolbox.test.js`, the
  new calls in `data.test.js`, the two-account case, the undo and the edit of the sketch in
  `mock-data.test.js`.
- 2026-10-22 — docs: `docs/development.md` (the voice paragraph names the undo and edit summaries, the UI
  section the undo button, the chips' controls and the account, `toolbox.js` and the mock, the IPC
  commands `toolbox`, `apply_set`, `undo_set`, `edit_set` and `clean_saved_variables`), `README.md` (the
  toolbox feature paragraph and the usage line of the window).

## Deviations from the plan

- The edit is not behind the running-client guard: it writes `sets.json` only, which the client never
  reads, and `sets::edit_set` and `mrgl set enable|disable|drop` (073) run while the client runs. Refusing
  it in the window alone would make the window and the command line disagree; the stand-in client test
  checks that the edit goes through with `WTF` unchanged, while the apply, the undo and the cleanup are
  refused. The edit is behind the one-write-at-a-time rule.
- *Not in this change* names no new core function; five small ones came with it, all over 073's functions:
  the undo and edit summaries and the not-installed warning (so the window prints the command line's lines
  from the core instead of a third copy, as task 086 did for the apply), `AddonSet::counted` (the count
  both summaries use) and `sets::undo_record` (the read-only record, so the undo button can say what it
  puts back).
- The account folder's name is in the window's answer for the twins only: the pick cannot tell them apart
  without it, and every other character stays as 074 left it.

## Verification

- 2026-10-22 — `cargo test --locked`: every suite green, among them the 12 of
  `crates/mrgl-app/tests/toolbox.rs` (the four new ones and the stand-in client test), the 19 of
  `crates/mrgl-core/tests/sets.rs`, the 128 of `crates/mrgl-cli/tests/cli.rs` (the undo and edit lines
  unchanged now that they come from the core) and `ui_smoke.rs`, whose `node --test` runs the 78 Node
  tests of `tests/ui/`.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check`: clean;
  `tasks_check` without `write`: only the stale README index, which is the main session's.
- 2026-10-22 — The window (`target/debug/mrgl-app.exe`, `MRGL_SERIOUS=1`) against a made-up flavor in a
  temporary folder (`MRGL_ROOT` and `MRGL_HOME` set, the addon-sets fixture's `WTF` with a second account
  holding a twin of `Fixture Realm/Mrgltest`, two sets saved with `mrgl set save --character`), driven
  over the web view's debug port: the target picker offered `Every character (3)`, `Mrgltest (Fixture
  Realm, account MRGLSETS)`, `Grrlwave (Second Realm)` and `Mrgltest (Fixture Realm, account MRGLTWIN)`;
  "Apply raid" to the twin answered `applied set raid to Fixture Realm/Mrgltest of _forever_: 4 addon lines
  changed` and changed the twin's `AddOns.txt` alone; the undo line named it and "Undo raid" answered
  `undid the apply of set raid to _forever_ (…): 1 AddOns.txt put back`, with every file under `WTF` equal
  by SHA-1 to before the apply; a second undo answered `nothing to undo in _forever_: …`. The `RaidFrames`
  chip toggled it to disabled and the `QuestArrows` `×` dropped it (`dropped QuestArrows from set raid of
  _forever_: 3 addons (2 enabled, 1 disabled); no AddOns.txt was touched`), the other set equal and `WTF`
  untouched. With the stand-in client (a copy of the system's `ping.exe` in the flavor folder) running, the
  panel named it, the apply to the twin and the undo answered `the client is running in _forever_
  (client-stand-in.exe, pid …): close the game first, nothing under WTF is written while it runs` with
  `WTF` unchanged, and a chip's toggle went through.
- 2026-10-22 — The sketch (`ui/index.html` with `mock-data.js`, served locally, in a headless browser):
  the toolbox tab in the light and the dark theme, the twin's option with its account, "Apply levelling"
  to it, the undo line, "Undo levelling" and its line `Made-up Realm/Mrglina (account MOCKTWIN):
  AddOns.txt put back as it was`; theme tokens only.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
