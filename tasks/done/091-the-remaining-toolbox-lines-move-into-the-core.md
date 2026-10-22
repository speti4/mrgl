---
size: S
origin: done/086-one-set-and-savedvars-summary-in-the-core.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
changelog: housekeeping
---

# 091 — The remaining toolbox lines move into the core: the set list note, the not-installed warning and the two-account error's words

## Context

The set list note (`2 sets of _forever_ in …`) and the apply's not-installed warning move into the
core as the summaries of 086 did, with the command line and the window's toolbox (085) both printing them
from there; and the core's two-account error names the account to pick without the command line's
`--account` flag, so the window can show the core's words.
Why: 086 moved the three summaries and left these two lines in both callers, and the two-account error ends
with a flag the window never has.
Done when: one function per line in the core with both callers, the CLI and the window tests assert the same
text, and the two-account error reads the same from both without naming a flag.

Decided 2026-10-22: a branch of its own, in parallel with the wave-7 follow-ups of the command line (088 to 090).

## Docs that move

- `docs/development.md`: the toolbox summaries in the core, if it lists them

## Not in this change

- The toolbox features themselves (085, 086); any new toolbox line

## Steps

1. [x] The core: one function for the set list note and one for the not-installed warning, next to the
       summaries of 086; the two-account error names the account to pick without a flag
2. [x] The command line and the window's toolbox print the core's lines; the CLI keeps its flag hint next to
       the core's words where it needs one
3. [x] Tests: the CLI tests and the window tests (Rust and the Node UI tests) assert the same text

## Log

- 2026-10-22 — promoted
- 2026-10-22 — core (`crates/mrgl-core/src/sets.rs`): `SetsFile::summary(path, folder_name)`, the set list
  note as a `(Kind, String)` pair (`` no sets of _forever_ yet: `mrgl set save <name>` saves one ``, a
  nothing; `1 set of _forever_ in …`, `2 sets of _forever_ in …`, a success, over `voice::count_noun`);
  `Applied::not_installed_warning(folder_name)`, the apply's warning as an `Option<String>`, the form of
  `Edited::not_installed`. `Error::AmbiguousCharacter` carries `by: sets::CharacterPick` (`Realm`,
  `Account`) in place of a `&'static str` hint; the account's words are now `the same realm is under more
  than one account: pick the account too`, no flag; the realm's stay `name one with --realm`.
- 2026-10-22 — callers: `mrgl set list` and the window's `toolbox::load` (the note under the set picker)
  print `SetsFile::summary`; `report_applied` and `toolbox::apply` print `Applied::not_installed_warning`;
  the copies are gone from `crates/mrgl-cli/src/main.rs` and `crates/mrgl-app/src/toolbox.rs`. The command
  line's `set_error` (for `set save` and `set apply`) adds `, with --account` after the core's words of the
  two-account error; the window shows the core's words as they are. The sketch's `mock-data.js` writes the
  same words, and its set note counts `1 set` and says when there is none.
- 2026-10-22 — tests: the core's `the_set_list_note_counts_the_sets_or_says_how_to_save_one`,
  `the_not_installed_warning_names_the_missing_addons_of_an_apply` and `the_two_account_hint_names_no_flag`
  (no `--` in the error, the realm's hint unchanged); `crates/mrgl-core/tests/sets.rs` with the new words;
  `tests/cli.rs` asserts the two-account error as the core's words plus `, with --account`;
  `crates/mrgl-app/tests/toolbox.rs` asserts the whole two-account line as the core writes it, and the new
  `the_set_list_note_reads_as_on_the_command_line` the one-set and the no-set note next to the two-set
  note the existing test asserts; the sketch's Node test asserts the whole two-account line, the whole
  not-installed warning and the two-set note.
- 2026-10-22 — docs: `docs/development.md`, the `set` section (the two-account error and the flag the command
  line adds), the voice section (where the shared lines live) and the `toolbox` and `apply_set` commands
  (the core's function named).

## Deviations from the plan

- `set save`'s own not-installed warning (`kept in the set all the same`) stays in the command line: the
  window saves no set, so it has one caller.

## Verification

- 2026-10-22 — `cargo test --locked`: every suite green, among them the `mrgl-core` unit tests (the three
  new ones), `crates/mrgl-core/tests/sets.rs`, `crates/mrgl-cli/tests/cli.rs` (135, the set list, apply and
  two-account tests among them), `crates/mrgl-app/tests/toolbox.rs` (13, with
  `the_set_list_note_reads_as_on_the_command_line`) and `ui_smoke.rs`, whose `node --test` runs the 80 Node
  tests of `tests/ui/`.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check`: clean;
  `tasks_check` without `write`: only the stale README index, which the main session regenerates.
- 2026-10-22 — the lines the CLI and the window print for the same outcome are one text: `2 sets of
  _forever_ in …` and `` no sets of _forever_ yet: `mrgl set save <name>` saves one `` on both, the apply's
  `set solo names addons that are not installed in _forever_, their lines are written all the same: …` on
  both, and the two-account error ends `the same realm is under more than one account: pick the account
  too` in the window and `… pick the account too, with --account` on the command line.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
