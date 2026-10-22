---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 027 — Per-character addon sets

## Context

A set (014) applied to one chosen character's `AddOns.txt` instead of every character of the flavor,
with the character picked by name from the `WTF` account folders; refused while the game runs (023).
014 applies a set to every character of the flavor (decided 2026-10-21); a raid character and a
levelling alt on the same flavor want different sets.
Done when: a fixture flavor with two characters switches one and leaves the other's file byte for byte.

Decided 2026-10-22: second of the chain 014 → 027; a `--character <name>` flag on `set apply` (a name
that matches two realms is refused with the realms listed, `--realm` narrows it).

## Docs that move

- `README.md` ("What it will do") and `docs/development.md`: the flag and the character rule.

## Not in this change

- A per-character set definition (a set is per flavor, the target is per character); the window.

## Steps

1. [x] The character list from the `WTF` account folders (account, realm, character) in the core
2. [x] `set apply <name> --character <name> [--realm <realm>]` writes one `AddOns.txt`; an ambiguous name
       is refused with the realms listed; the `WTF` write guard holds
3. [x] Tests on the fixture with two characters: one switched, the other byte for byte; the ambiguity
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 014 → 027
- 2026-10-22 — The character list itself came with 014 (`sets::characters`, `characters_in`: every folder
  `WTF\Account\<account>\<realm>\<character>\`, sorted, the account's `SavedVariables` no realm). On top of
  it, `sets::Target` (`EveryCharacter`, the default, or `Character { name, realm }`) and
  `sets::find_character`: the name and the realm compared ignoring case; one match is the character, none
  is `Error::CharacterNotFound` with every character listed as `Name (Realm)`, several are
  `Error::AmbiguousCharacter` with the realms listed and the hint `name one with --realm`; when one realm
  and name sit under two accounts, the candidates are `Realm (ACCOUNT)` and the hint says `--realm` cannot
  tell them apart. Both errors are appended at the end of `Error`.
- 2026-10-22 — `save_set` and `apply_set` take a `&Target`; `targets` resolves it before the guard, so an
  unknown or ambiguous name is reported without a process check, and the guard of 023 runs before the
  first write for one character as for all. `Saved::characters` is now the list of characters read
  instead of their count, so a summary can name the one character.
- 2026-10-22 — `mrgl-cli`: `--character` and `--realm` (which requires `--character`) on `set apply` and
  `set save`; the progress line names the character as given (`applying set raid to the character
  grrlwave of _forever_`), the summary names the one found (`applied set raid to Second
  Realm/Grrlwave of _forever_: 3 addon lines changed`, `set raid already matches Second
  Realm/Grrlwave of _forever_, nothing written`); `--json` carries the one character under `characters`.
- 2026-10-22 — Docs: README (the `--character` usage line and the paragraph on sets), docs/development.md
  (the CLI line in "Layout" and a paragraph on the character rule, `Target` and `find_character`), the
  module docs of `sets.rs` and the CLI.
- 2026-10-22 — CodeQL flagged the account folder's name in the data lines of `set apply` as cleartext
  logging of sensitive information (on an older install the folder is the login name): the plain lines and
  summaries name `<realm>/<character>` only; the account stays in `--json` and in the one error that needs
  it, one realm and name under two accounts.

## Deviations from the plan

- Step 1 landed with 014, whose apply to every character needs the same list; this task added the
  selection on top of it.
- `--character` (and `--realm`) also works on `set save`: with 014's save, an addon two characters
  disagree on is left out, so a raid set is taken from the raid character alone. The set stays per flavor.
- A realm name under two accounts is a case the realm flag cannot narrow; it is refused with the accounts
  named instead of picking one.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green. `crates/mrgl-core/tests/sets.rs` on a copy of the
  `addon-sets` tree: an apply to `grrlWAVE` changes only Grrlwave's file (its bytes as expected, its
  modification time new) and Mrgltest's file is byte-equal; an apply to `Mrgltest` on `fixture realm`
  changes only Mrgltest's file and Grrlwave's stays byte-equal; a save from `Mrgltest` alone takes its four
  addons with no conflict; with a second `Mrgltest` under `Second Realm`, the bare name is refused for an
  apply and for a save with `character mrgltest matches 2 characters (flavor forever): Fixture Realm,
  Second Realm; name one with --realm` and `WTF` is byte-equal, `Third Realm` gives `character not found:
  Mrgltest on Third Realm (flavor forever); the characters: Mrgltest (Fixture Realm), Grrlwave (Second
  Realm), Mrgltest (Second Realm)`, and `Second Realm` writes that Mrgltest's file only; one realm under
  two accounts names the accounts; with the stand-in client running an apply to one character is refused
  and `WTF` is byte-equal.
- 2026-10-22 — end to end through the exe (`crates/mrgl-cli/tests/cli.rs`): `set save raid --character
  Mrgltest` (`saved set raid of _forever_ from Fixture Realm/Mrgltest: 4 addons (3 enabled, 1
  disabled)`), `set apply raid --character grrlwave` (the progress, data, warning and summary lines, the
  other character's file byte-equal, Grrlwave's exact bytes), a second apply says nothing was written, the
  ambiguous name exits 1 with the realms listed and writes nothing, `--json … --realm "Second Realm"`
  writes that character only, `--realm` without `--character` exits 2; with the stand-in client running,
  `set apply raid --character Grrlwave` exits 1 with the refusal line and `WTF` is byte-equal.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90
  check --all-targets --locked` and `python -I scripts/changelog.py --check`: all green. No manifest
  changed, no new crate. No round on a real game folder: the tests run in temporary folders.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
