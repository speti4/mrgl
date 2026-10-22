---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 014 — Addon sets with enable and disable

## Context

Named sets of addons per flavor (raid, levelling, minimal), stored under the flavor's sidecar folder
(`sets.json`), switched with one command or click by rewriting every character's `AddOns.txt` under `WTF` in
the client's own format, never by deleting folders; refused while the game runs (023). Decided 2026-10-21: a
set applies to every character of the flavor; one chosen character is task 027. Switching a dozen addons by
hand before a raid is the second most common chore.
Done when: a set switch changes exactly the listed addons in a fixture flavor, the game's own files are left
in the format the client wrote, and a switch is refused while the client runs.

Decided 2026-10-22: first of the chain 014 → 027 on one branch; the CLI surface is `mrgl set` with
`save <name>` (from the current `AddOns.txt` state), `apply <name>`, `list` and `remove <name>`, or the
verbs the CLI's existing commands make natural; the window's click is 010's or a later task's.

## Docs that move

- `README.md` ("What it will do") and `docs/development.md` ("Layout", the home folder's files): the
  `set` command, `sets.json`, and the `AddOns.txt` rule (the second `WTF` write of D-002 and D-003).

## Not in this change

- One chosen character (027, next in the chain); the window's set switch; the SavedVariables cleanup (025).


## Steps

1. [x] The `AddOns.txt` reader and writer in the core (every account and character under `WTF`), round-
       tripping the client's format byte for byte when nothing changes; the `WTF` write guard of 023 before
       every write
2. [x] `sets.json` under the flavor's sidecar folder: save, apply, list, remove; an apply changes exactly
       the listed addons and leaves the rest as they are
3. [x] The CLI command with the voice's lines and `--json`
4. [x] Tests on a fixture `WTF` with two characters: the round-trip, an apply, the refusal while the
       stand-in client runs
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 014 → 027
- 2026-10-22 — `crates/mrgl-core/src/sets.rs`, a new module: `AddonsTxt` reads the file as bytes, line by
  line with each line's ending (CRLF, LF, none on a last line), and `to_bytes` gives back exactly the bytes
  read; an addon line is `Name: enabled|disabled` with spaces allowed around either part and a byte order
  mark skipped, every other line is kept as it is. `set_state` replaces only the state word of a line whose
  state differs (a line with the same state in another case stays), and appends `Name: state` with the
  file's first line ending (CRLF in an empty file) when the file does not name the addon, closing an open
  last line first. `characters_in` walks `WTF\Account\<account>\<realm>\<character>\` (the account's
  `SavedVariables` is no realm; files, links and junctions are passed over; a missing `WTF` is no
  character).
- 2026-10-22 — The sets file: `<home>\<flavor-id>\sets.json` (`format` `mrgl-sets`, `schema` 1, `flavor`,
  `sets` by name with `saved_at` and `addons`), written through a temporary file. It is the user's data, so
  an unreadable file, another format, schema or flavor, or an addon name with a colon, a control character
  or a path separator is `Error::SetsUnreadable` and the file is never overwritten. A set name is trimmed,
  1 to 64 characters, no control character; a lookup takes the exact name, else the one name equal
  ignoring ASCII case.
- 2026-10-22 — `save_set` reads every character's file; an addon with one state across the files that name
  it joins the set, one the characters disagree on is left out and named in `conflicts`; a save of an
  existing name replaces that set; it reads `WTF` only, so it goes on while the client runs. `apply_set`:
  the set, the characters, then `client::ensure_client_not_running`, then every new content worked out
  before the first write; only the files that change are written (temporary file renamed over the old
  one), and a failed write puts the files already written back (`Error::SetApplyFailed` says whether that
  worked). The result names each character's changed lines, a created file, and the set's addons that have
  no folder under `Interface\AddOns` (a warning; the line is written all the same). `remove_set` and
  `find_set` touch `sets.json` only. Six errors appended at the end of `Error`: `SetsUnreadable`,
  `SetName`, `SetNotFound`, `NoCharacters`, `NothingToSave`, `SetApplyFailed`.
- 2026-10-22 — `mrgl-cli`: `mrgl set save|apply|list|show|remove`, each with `--flavor`; the plain lines
  (`saved set raid of _forever_ from 2 characters: …`, one data line per character on apply, `applied set
  raid to 2 characters of _forever_: 4 addon lines changed`, the nothing-to-do line `… already matches
  every character …, nothing written`), warnings for the conflicts and the addons that are not installed,
  and `--json` documents of kind `set-save`, `set-apply`, `set-list`, `set-show` and `set-remove`.
- 2026-10-22 — A second synthetic `WTF` tree, `crates/mrgl-core/tests/fixtures/addon-sets/WTF/`: one
  account, two characters on two realms, one file with CRLF endings, the other with LF, a line the client
  never wrote and no last line break; the two disagree on two addons.
- 2026-10-22 — Docs: README ("What it will do": the toolbox bullet; the `set` usage lines; a paragraph on
  sets); docs/development.md ("Layout": the `sets` module and the `set` command, a paragraph on the sets
  file and the `AddOns.txt` rule as the second `WTF` write of D-002 and D-003, `sets.json` among the home
  folder's files, the new fixture); the fixtures README; the module docs of `lib.rs` and `backup.rs`.

## Deviations from the plan

- A fifth verb, `set show <name>`, prints a set's addons as `Name: state` lines: `set list` gives counts
  only, and a set is otherwise unreadable without opening `sets.json`.
- The character walker of 027's first step came with this task: an apply to every character needs the
  same list. 027 adds the selection by name and realm on top of it.
- A character folder without an `AddOns.txt` gets one with the set's lines (CRLF): the client reads an
  absent file as every addon at its default state, so the set would otherwise not hold for that character.
- The fixture is a second tree (`addon-sets/`) instead of a second character in 013's `wtf/` tree, whose
  file and folder counts the backup tests assert.
- An apply is all or nothing over the files it writes: a failed write puts the files already written back.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green. `crates/mrgl-core/tests/sets.rs` on a copy of the
  `addon-sets` tree: both characters are listed (`Fixture Realm/Mrgltest`, `Second
  Realm/Grrlwave`, the account's `SavedVariables` not a realm) and both `AddOns.txt` round-trip byte for
  byte; a save keeps `BagSorter` and `RaidFrames` enabled, leaves out `BossTimers` and `QuestArrows`, writes
  `sets.json` under `home\wowf\` and leaves `WTF` byte-equal; a save under the same name in another case
  replaces the set; an apply of a four-addon set writes exactly the expected bytes into both files (the
  CRLF file gets `BagSorter: disabled` in place and `NewMeter: enabled` at the end, the LF file gets its
  last line closed and two lines appended, the unknown line kept), every other file under `WTF` is
  byte-equal, `not_installed` names the two addons without a folder; a second apply changes nothing and
  leaves both files' modification times as they were; a character without `AddOns.txt` gets one; with the
  stand-in client running, the apply is refused with the documented line and `WTF` is byte-equal, a save
  goes on, and after the client ends the apply goes through; a read-only second file makes the apply fail
  with `every AddOns.txt is as it was` and the first file is byte-equal again; an unknown set, a broken and
  a hostile `sets.json` (never overwritten) and a flavor without `WTF` give their documented errors. Unit
  tests in `sets.rs` cover the round trip of odd files (a BOM, no last line break, bytes that are not
  UTF-8, bare line breaks), the entry parsing, the in-place change, the append and the name checks.
- 2026-10-22 — end to end through the exe (`crates/mrgl-cli/tests/cli.rs`): `set list` on no file, `set save
  everyday` (the conflict warning and `saved set everyday of _forever_ from 2 characters: 2 addons (2
  enabled, 0 disabled)`), `set list`, `set show raid`, `set apply raid` (one data line per character, the
  not-installed warning, `applied set raid to 2 characters of _forever_: 4 addon lines changed`, the files'
  exact bytes), a second apply (`set raid already matches every character of _forever_ (2 characters),
  nothing written`), `set remove raid`, then `set apply raid` exits 1 with `set not found: raid (flavor
  forever); the saved sets: everyday`; the `--json` documents of save, apply, list and an error; `set apply`
  with the stand-in client running exits 1 with the refusal line and `WTF` byte-equal.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90
  check --all-targets --locked` and `python -I scripts/changelog.py --check`: all green. No manifest
  changed, no new crate. No round on a real game folder: the tests run in temporary folders.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
