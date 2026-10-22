---
size: S
origin: done/064-blizzard-addons-are-not-missing-dependencies.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 080 — Housekeeping after the M1 fixes: one `Blizzard_` prefix, the doctor's formatter alias, the voice sample in the docs

## Context

`dependencies::CLIENT_PREFIX` and `savedvars::CLIENT_PREFIX` become one shared constant and helper;
the doctor calls `zip_entry::size_text` directly and the `bytes_text` alias import goes; and the planned voice
sample in `README.md` and `docs/nerglish.md` shows the per-addon progress lines of the update check (068) as
the command line and the window (010) print them. Two copies of the `Blizzard_` rule will drift apart (064,
071), the alias was a bridge for the parallel branches of wave 5, and the sample predates the progress lines.
Done when: one constant with both callers, no alias import, the sample matches a real `update --all` run's
lines, and every existing test is unchanged.

Decided 2026-10-22: second of the chain 076 → 080 on one branch, in wave 6.

## Docs that move

- `README.md` and `docs/nerglish.md`: the voice sample shows the progress lines of `update --all`.
- `docs/development.md`: the module that holds the shared `Blizzard_` constant, where the rule is named.

## Not in this change

- Any change to the dependency or SavedVariables rules themselves (076 changes the dependency rule); the
  window's output.

## Steps

1. [x] One shared `Blizzard_` constant and helper in the core, called by `dependencies` and `savedvars`
2. [x] `doctor.rs` calls `zip_entry::size_text` directly, the `bytes_text` alias import goes
3. [x] The voice sample in `README.md` and `docs/nerglish.md` matches the lines of a real `update --all` run
4. [x] Every existing test unchanged and green; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 076 → 080
- 2026-10-22 — `scan::CLIENT_ADDON_PREFIX` (`Blizzard_`) and `scan::is_client_addon` (ASCII
  case-insensitive, the body `dependencies::is_client_addon` had) are the one rule; `dependencies` imports
  the helper, `savedvars` calls it on its lower-case key instead of `starts_with` on the lower-cased
  prefix (the same answer: the prefix is ASCII); both `CLIENT_PREFIX` constants are gone. The modules' docs
  point at the shared rule.
- 2026-10-22 — `doctor.rs` imports `zip_entry` and calls `zip_entry::size_text` at its two call sites; the
  `bytes_text` alias and its comment are gone.
- 2026-10-22 — the voice sample: a scratch game folder outside the repo (a `.build.info` row for `wowf`
  at `1.60.1.70245`, `Questie` 1.2.2 from `github:Questie/Questie` and `Plain` 1.0.0 from
  `github:Example/Prerelease` in its sidecar), `mrgl update --all --flavor forever --yes` with `MRGL_HOME`
  in the scratch folder, `MRGL_DISCOVERY=off` and `MRGL_HTTP_STUB` at the fixture answers; its eighteen
  lines (the run's progress line, each addon's `checking … (i of 2)` and result line, the update's
  `updating`, `downloading`, `installing` and `updated` lines, the summary) are the sample in `README.md`
  and in `docs/nerglish.md` as printed. `docs/nerglish.md` adds that the window's status bar shows the same
  lines one after the other, its `updating` line with the addon's place in the run, and lists `update`'s
  result lines among the data lines that stay plain; its two-line example takes the run's summary.
- 2026-10-22 — `docs/development.md`: the module map names `scan` as the home of the `Blizzard_` rule, and
  the dependency check and the SavedVariables paragraphs point at `scan::is_client_addon`.

## Deviations from the plan

- The shared constant and helper live in `scan`, the module of the addon folders, rather than in a module
  of their own: the rule is about addon folder names, which both callers compare.
- One test body changed by name only: the doctor's `a_byte_count_reads_for_a_human` calls
  `zip_entry::size_text` where it called the alias; its four values are unchanged.
- The sample in `README.md` was headed *Planned output (M1)*; it is now a real run, introduced as one, and
  runs with `--yes`, since without it the command asks first.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green with no expectation changed (core unit 186,
  `tests/savedvars.rs` 6, `tests/dependencies.rs` 2, CLI `tests/cli.rs` 119, the window's 13
  `flavor_list` and 8 `update_actions` tests, the UI smoke test with Node on `PATH`). `cargo fmt --check`,
  `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check --all-targets --locked` and
  `python -I scripts/changelog.py --check` green. `grep` over `crates/` finds no `CLIENT_PREFIX` and no
  `bytes_text`.
- 2026-10-22 — the `update --all --yes` run of the Log printed the eighteen lines of the sample, exit code
  0; its plain lines have the shapes the CLI tests assert, the `updating`, `downloading`, `installing` and
  `updated` lines word for word as in `update_all_with_yes_updates_through_the_one_path_and_counts_the_rest`,
  the `checking` and result lines as in `update_dry_run_reports_every_state_of_a_fixture_sidecar_and_writes_nothing`.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
