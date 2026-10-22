---
size: S
origin: done/078-the-windows-cached-view-walks-the-releases-and-keeps-the-last-answer.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 087 — The command line's update check writes the last-check file the window reads

## Context

`mrgl update --all` and `mrgl update <name>` record their check in `<flavor-id>\last-check.json` through
`mrgl_core::last_check` as the window's check does (078), so a source gone, a failed check or a budget stop
found on the command line shows in the window, and a source found back clears it there.
Why: the file is written by the window only, so after `mrgl update` finds a gone source back while the
version stays the same, the window still shows it gone until its own next check.
Done when: a CLI test's check writes the file with the kept states, a second check that finds the source
back clears the entry, and the window's flavor list test reads what the command line wrote.

Decided 2026-10-22: runs last on the branch of 082 and 083, since it changes the same `update` command.

## Docs that move

- `docs/development.md`: the last-check file's writers (the window and the command line) and the `update` command
- `README.md`: the kept answers paragraph, if it says the window alone writes them

## Not in this change

- The window's check and the rules of `last_check` (078); a new kept state

## Steps

1. [x] `mrgl-cli/src/main.rs`: `update --all` and `update <name>` write the kept states of their check through
       `mrgl_core::last_check` after the run, under the same rules as the window (a failed write is a warning)
2. [x] Tests: a CLI test's check writes the file with the kept states and a second check that finds the source
       back clears the entry (`tests/cli.rs`); the window's flavor list test reads a file the command line's
       shape wrote (`crates/mrgl-app/tests/flavor_list.rs`)
3. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — core: `last_check::record_check(home, flavor, state, addons)` reads the kept answers,
  records an online check's addons over the sidecar (`LastCheck::record`, the rules of 078 unchanged) and
  writes the file, the one call for both writers. The window's `actions::run` keeps its own three lines
  (read, record, write), which do the same.
- 2026-10-22 — CLI: `update --all` and `update <name>` call `record_check` right after the check, before
  the confirmation and on `--dry-run` too (a dry run is a check), with the sidecar as it is before any
  update; a failed write is a warning through the voice, `the answers of this check are not kept for the
  window: <error>`, in the text and in the JSON's `warnings`. An update installed afterwards changes the
  entry's version, so a kept answer of the old version no longer applies, as on the window's update.
- 2026-10-22 — tests: `tests/cli.rs` `update_keeps_a_gone_source_for_the_window_until_it_answers_again`
  (offline with nothing cached both addons are kept as `failed`; online the 404 replaces one with
  `source-gone` and the addon up to date drops its own; `update Gone` against a stub copy whose
  `Example/Missing` serves a release list with the installed version answers `up to date` and empties the
  file) and `update_warns_when_the_last_check_file_cannot_be_written_and_goes_on` (a folder in the file's
  place); core unit `record_check_reads_records_and_writes_in_one_call`; the window's
  `tests/flavor_list.rs` `the_rows_read_the_last_check_file_the_command_line_writes` (a new test
  function: the file as `mrgl update` wrote it for a 404 shows `source gone` on the Forever `Questie` row,
  and the row is `up to date` again once the entry is gone).
- 2026-10-22 — docs: `docs/development.md` (the `update` command records its check, the last-check file's
  two writers in the `flavors` command's offline rule and in the mrgl home's files), `README.md` (the kept
  answers paragraph: the window's own check or `mrgl update`'s).

## Deviations from the plan

- The write happens right after the check, before the confirmation and the installs, and on `--dry-run`
  too, as the window records its check before its updates: the file is a cache like the release list,
  which a dry run writes as well.
- The window keeps its own read, record and write rather than calling `record_check`, so this change
  leaves the window's crate alone apart from the new test.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (`tests/cli.rs` 133, was 131; core unit 190, was
  189; `tests/flavor_list.rs` 15, was 14; `tests/update_actions.rs` 10 and the other tests unchanged).
  `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.
- 2026-10-22 — `mrgl update --all --dry-run --flavor forever` in a temporary game folder and `MRGL_HOME`
  over a sidecar with one addon from `github:Example/Missing` against the fixture stub: `Gone  1.2.3
  source gone: GitHub answered 404 … no such repository, or it is private`, `dry run: 1 source gone;
  nothing written`, and `<home>\wowf\last-check.json` with `format` `mrgl-last-check`, `schema` 1,
  `flavor` `wowf` and `Gone` kept as `source-gone` for `github:Example/Missing` 1.2.3 with the 404 line as
  its `detail`. No real game folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
