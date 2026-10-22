---
size: S
origin: done/074-window-toolbox-sets-and-savedvariables.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
changelog: housekeeping
---

# 086 — One summary line of `set apply` and `savedvars` in the core, shared by the command line and the window

## Context

The summary lines of `set apply` and of the SavedVariables listing and cleanup move into the core (`sets` and
`savedvars`), and the command line and the window's toolbox (074) both print them from there; the orphan
count agrees with its verb (`1 orphan (…) has no addon folder`, `2 orphans … have`).
Why: 074 copied the command line's summary text into the window, two copies will drift apart, and both say
`1 orphan … have`.
Done when: one function per summary in the core with both callers, the CLI and the window tests assert the
same lines, and the one-orphan line reads `has`.

Decided 2026-10-22: runs first on a branch shared with 085, so the window's new toolbox actions build on the
shared summaries.

## Docs that move

- `docs/development.md`: where the toolbox's summary lines live (the core), named in the voice or the toolbox
  section

## Not in this change

- New toolbox actions in the window (085); any other line the window copied from the command line

## Steps

1. [x] `crates/mrgl-core/src/sets.rs` and `savedvars.rs`: one function per summary (the apply summary, the
       SavedVariables listing summary, the cleanup summary) that returns the voice line, with the orphan
       count's verb agreeing with the number: `Applied::summary`, `Listing::summary` and
       `Cleaned::summary`, each a `(Kind, String)` as `UpdateCheck::summary` of task 010 is, over one
       `voice::count_noun`
2. [x] `crates/mrgl-cli/src/main.rs` and `crates/mrgl-app/src/toolbox.rs` print the core's lines, the copies gone
3. [x] Tests: the core's summaries with one and with two orphans; the CLI and the window tests assert the same
       lines (`tests/cli.rs`, `crates/mrgl-app/tests/toolbox.rs`, the UI's Node tests if the text is checked)
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — core: `voice::count_noun` (`1 orphan`, `2 orphans`, `0 orphans`);
  `sets::Applied::summary(target, folder_name)` (the one character's `<realm>/<name>` when the target named
  it, else the count; `already matches …, nothing written` when no line changed);
  `savedvars::Listing::summary(folder_name, cleanup)` with `savedvars::CleanupBy` (`Command` names
  `` `mrgl savedvars --clean` ``, `Window` names `the cleanup`), the verb and the pronoun agreeing with the
  orphan count (`1 orphan (12 B) has no addon folder: … moves it`, `2 orphans … have …: … moves them`);
  `savedvars::Cleaned::summary(folder_name)`. Each returns the kind and the plain line; the caller says it
  through its own voice.
- 2026-10-22 — CLI: `report_applied` and `savedvars_command` print the core's lines; `count_orphans` and
  `count_files` are gone from `main.rs`. Window: `toolbox::load`, `apply` and `clean` print the same lines
  through `said` (the core's pair as a `Message`); `listing_summary` and `apply_summary` are gone from
  `toolbox.rs`. The sketch's `mock-data.js` agrees the verb with the orphan count too.
- 2026-10-22 — tests: the core's `the_listing_summary_agrees_with_the_orphan_count` (one and two orphans,
  none, an empty `WTF`, both `CleanupBy`), `the_cleanup_summary_names_the_zip_or_says_nothing_moved`,
  `the_apply_summary_names_the_one_character_or_counts_them`, `a_count_takes_its_plural_but_for_one`;
  `tests/cli.rs` `savedvars_says_has_for_one_orphan` and `tests/toolbox.rs`
  `one_orphan_reads_has_as_on_the_command_line` (the one-orphan listing and its cleanup on each side, next
  to the two- and three-orphan lines the existing tests assert); the sketch's Node test checks the
  two-orphan line.
- 2026-10-22 — docs: `docs/development.md`, the voice section (where the shared summary lines live) and
  the `toolbox`, `apply_set` and `clean_saved_variables` commands (the core's function named).

## Deviations from the plan

- Decided 2026-10-22: the listing summary takes the way to clean up from its caller (`CleanupBy`): the
  command line names `` `mrgl savedvars --clean` ``, the window its cleanup button, as both did before; the
  rest of the line is one text in the core.
- `voice::count_noun` writes `0 orphans`, where the window's `list::count_noun` writes `no orphans`: the
  summaries never count zero characters or orphans (a zero is its own line), and the command line's lines
  stay as they were.

## Verification

- 2026-10-22 — `cargo test --locked`: every suite green, among them the 193 unit tests of `mrgl-core`
  (the four new summary and count tests), the 128 of `crates/mrgl-cli/tests/cli.rs` (with
  `savedvars_says_has_for_one_orphan`), the 9 of `crates/mrgl-app/tests/toolbox.rs` (with
  `one_orphan_reads_has_as_on_the_command_line`) and `ui_smoke.rs`, whose `node --test` runs the 72 Node
  tests of `tests/ui/`.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check`: clean;
  `tasks_check` without `write`: no finding.
- 2026-10-22 — the lines the CLI and the window print for the same outcome are one text: the one-orphan
  listing reads `… 1 orphan (… B) has no addon folder: … moves it into a backup zip` on the command line
  (naming `mrgl savedvars --clean`) and in the window (naming the cleanup), the cleanup
  `moved 1 orphan of _forever_ (1 file, … B) into …` on both.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
