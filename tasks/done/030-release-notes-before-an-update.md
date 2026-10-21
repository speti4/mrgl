---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 030 — Release notes before an update: the GitHub release body in one line

## Context

The first line of the GitHub release body in `update --dry-run` and in the window's row (010), with the
release page's link; the body comes with the release list the source (003) already fetches. Knowing
what an update changes before taking it, and a link to the rest when one line is not enough.
Done when: a fixture release with a body shows its first line and the link, and an empty body shows
nothing.

Decided 2026-10-21: the pick carries the notes (`notes`: the first non-empty line of the release body,
Markdown markers and trailing whitespace stripped, cut at a documented length; `page`: the release's
`html_url`), both optional; `add --dry-run` shows them now, `update --dry-run` (006) and the window's
row (010) take them from the same pick when they land; the release list cache (003) keeps the body, so
nothing is fetched twice.

## Docs that move

- `docs/development.md` ("Layout"): the two fields on the pick and the first-line rule.
- `README.md`: the notes line of `add --dry-run`.

## Not in this change

- `update --dry-run` itself (006); the window's row (010); the full body or a pager.

## Steps

1. [x] The release type reads `body` and `html_url` from the list; the pick gets `notes` and `page` with
       the first-line rule and its unit tests (empty body, a body of headings only, a long line)
2. [x] `mrgl-cli`: `add --dry-run` shows the line and the link through the voice and in `--json`
3. [x] Fixture: a release list with a body and one without, under the folder stub's fixtures; the
       end-to-end test for both
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `Release` (`sources::github`) reads `body` and `html_url`, both optional (`#[serde(default)]`,
  so a missing field and `null` both read as `None`). `Pick` gains two fields at its end, `notes` and `page`,
  both `Option<String>` and left out of the JSON when `None`; `GitHub::resolve` fills them, an empty
  `html_url` counting as none.
- 2026-10-21 — the first-line rule is `sources::notes_line` with `NOTES_MAX_CHARS = 120`: the first line of
  the body that keeps text once leading `#`, `*`, `-`, `+`, `>` and trailing `#`, `*` are stripped with the
  whitespace around them (a rule `---` or a bare `##` counts as empty); a longer line keeps 119 characters,
  loses a trailing space, and ends with `…`. Unit tests: an empty body, a body of markers and blank lines
  only, a body of headings only (the first heading's text), a long line (cut by characters, not bytes, and
  at a word end), CRLF lines.
- 2026-10-21 — the release list cache (003) stores the API's list as raw JSON, so the body was already kept:
  confirmed by `the_cached_release_list_keeps_the_body` (the cached file carries `body`, the second run reads
  it after a 304 and gives the same notes).
- 2026-10-21 — `mrgl-cli`: `say_release_notes` prints `release notes: <line> (<page>)` after the pick line,
  through the voice with the neutral kind (`Mrgl.`), and nothing without notes; the `--json` document carries
  `notes` and `page` inside `pick`, which already serializes the whole `Pick`.
- 2026-10-21 — fixtures `Example/Notes` (a made-up body and a `release.json`) and `Example/NoNotes` (an empty
  body) under `crates/mrgl-core/tests/fixtures/releases/`; the core test and two CLI end-to-end tests cover
  both.

## Deviations from the plan

- The notes line prints whenever a pick is reported, with or without `--dry-run`: it sits right after the
  pick line, and an `add` without `--dry-run` still ends at task 040 today.
- `page` is kept on the pick even when the body is empty (the release page exists either way); only the text
  line needs `notes`.

## Verification

- 2026-10-21 — `cargo test --locked` green, among them `the_notes_line_is_the_first_line_with_text_and_its_markers_gone`,
  `an_empty_body_or_one_of_markers_only_has_no_notes`, `a_body_of_headings_only_gives_the_first_heading`,
  `a_long_line_is_cut_with_an_ellipsis`, `github::tests::the_pick_carries_the_notes_line_and_the_page`,
  `github_source::the_release_body_gives_the_notes_line_and_the_page`,
  `github_source::the_cached_release_list_keeps_the_body`,
  `cli::add_dry_run_shows_the_release_notes_line_and_the_page` and
  `cli::add_dry_run_of_a_release_with_an_empty_body_shows_no_notes`.
- 2026-10-21 — `mrgl --serious add github:Example/Notes --flavor forever --dry-run` against the fixture stub
  prints the pick line, then `release notes: Quest arrows point the right way again
  (https://github.com/Example/Notes/releases/tag/v2.1.0)`, then `dry run: nothing installed`; the same run for
  `github:Example/NoNotes` prints three lines and no notes line, and its `--json` pick has `page` and no
  `notes`.
- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
