---
size: S
origin: done/120-a-usage-guide-as-html-inside-the-release-zip.md
created: 2026-10-24
started: 2026-10-24
closed: 2026-10-24
plan: none
milestone: M5 Trust
changelog: housekeeping
---

# 122 — Tests of the usage guide generator's error paths

## Context

Unit tests of `scripts/usage-guide.py`'s rejections (a marker out of place, Markdown the page cannot render, a
link to a missing file, a render that is not deterministic) over a fixture README, run where the `--check`
runs in the `lint` job; the repo's first Python test file, in a shape its other scripts can follow.
Why: 120 verified those rejections by hand once; the generator runs in CI and in `release.ps1`, and a silent
change to a rejection would reach the zip.
Done when: each documented rejection has a test that fails without it, the tests run in CI, and CONTRIBUTING's
checks name the command.

Decided 2026-10-24: the standard library's `unittest` only (D-002, nothing to install), the fixture READMEs
made inside the tests or under a folder next to the script, never a copy of the real README.

## Docs that move

- `CONTRIBUTING.md`: the test command in "Checks before a commit"
- `docs/development.md`: "The usage guide" names the tests and how a new rejection gets one; the Layout
  bullet of `scripts/` if the tests live there

## Not in this change

- The generator's behavior (a rejection the tests reveal as wrong is a follow-up, not a fix here)
- Tests of `changelog.py`, `coverage-check.py` or `sync-issues.py` (the shape is set here, the others follow
  later if wanted)

## Steps

1. [x] The test module (`scripts/tests/test_usage_guide.py` or the name the docs decide) with `unittest`,
       importing the script as a module; one test per documented rejection, each asserting the exit and the
       README line number in the message, and one happy-path test over a small fixture README that renders
       deterministically
2. [x] The command (`python -I -m unittest discover -s scripts/tests` or the like) in the `lint` job next to
       the `--check` step, in CONTRIBUTING and in the docs
3. [x] Verification: the tests pass locally and in CI; each test fails when its rejection is disabled (tried
       once by hand, the Log says which)

## Log

- 2026-10-24 — promoted
- 2026-10-24 — `scripts/tests/test_usage_guide.py`, the repo's first Python test module: the standard library's
  `unittest` only (D-002). The script's name has a hyphen, so the module loads it from its path with
  `importlib.util.spec_from_file_location` and registers it in `sys.modules` (its dataclasses look their module
  up by name). `python -I -m unittest discover -s scripts/tests` finds it without a package or an
  `__init__.py`: discovery puts the start folder on `sys.path` itself, which isolated mode leaves alone.
- 2026-10-24 — The tests drive `main()` as `--check` does, with the module's `README` and `CARGO_TOML` swapped
  for temporary files and `sys.argv` set, so each one sees the exit code and the stderr line the lint job
  prints. The fixture READMEs are strings in the test; a helper wraps body lines in one region under one `##`
  heading, so a construct lands on a known README line. A link in a fixture names a file of the real checkout
  (`CONTRIBUTING.md`, `docs/`), since the generator resolves links against the folder it sits in.
- 2026-10-24 — 37 rejection tests: the markers (unreadable, unknown, a `section` without a title, a start
  inside a region, an end without a start, a section outside a region, a start without its end, an open code
  fence, no start marker), no `##` heading, a `####` heading, the blocks (table, block quote, numbered list,
  `*` and `+` items, nested list, indented code, thematic break, setext heading, raw HTML, another HTML
  comment, a construct on a continuation line), the inline constructs (an image with and without a link
  target, `*` and `_` emphasis, inline HTML, an autolink, a reference link, a construct in a list item and in
  a heading), a link to a missing file and one out of the repo, `Cargo.toml` without a version or a
  repository, and two renders that differ (`build` patched to return two pages). Each asserts exit 1, an
  empty stdout, the message prefix with the README line and the rejection's own words. 7 happy-path tests
  over a fixture README of two regions: `--check` passes with 2 sections, two renders are byte-equal, the
  headings and the contents, only the regions on the page, the inline Markdown and the `target\release\`
  rewrite, the links (a repo file, a folder, a page anchor, a README anchor, an outside address) and no
  external asset.
- 2026-10-24 — A sweep that took each of the 28 rejections out of the script in turn showed two gaps, both
  fixed in the tests: the block-level raw HTML rejection was masked by the inline one (the test now asserts
  `raw HTML: <line>`, the block message), and the inline `![` pattern had no test of its own (an image without
  a link target now has one).
- 2026-10-24 — The CI step is named for the helper scripts' tests, not for the guide alone, so a later test
  module under `scripts/tests/` runs with the same command.

## Deviations from the plan

- Step 1 asked each rejection test to assert the README line number. Six rejections carry none in the
  generator (a region or a code fence left open, no start marker, no `##` heading, and the two `Cargo.toml`
  ones, which name `Cargo.toml`); their tests assert the message as the generator writes it, `README.md:` (or
  `Cargo.toml:`) without a number. Naming the line where an open region or fence starts is a change of the
  generator, a follow-up (pull request).
- An open code fence inside a region turns the region's end marker into code, so the generator reports such
  a README as a region without its end; the fence's own message is reached only by a fence left open after the
  last region. The test covers both READMEs as they behave today.

## Verification

- 2026-10-24 — `python -I -m unittest discover -s scripts/tests` on Python 3.13 and 3.14: 44 tests, OK.
- 2026-10-24 — By hand: the table pattern removed from `UNSUPPORTED_BLOCKS` in the working copy, the suite run,
  the file restored with `git checkout`: `test_table` failed alone ("accepted: GUIDE.html renders from
  README.md (1 sections, 2 KB)"). Then a scratch script outside the repo took out each of the 28 rejections in
  turn (the 13 patterns of `UNSUPPORTED_BLOCKS` and `UNSUPPORTED_INLINE` and 15 `raise` statements, among them
  the determinism guard of `main()`), ran the suite and restored the file: after the two fixes in the Log,
  every mutation failed at least one test, the one named for it.
- 2026-10-24 — `python -I scripts/usage-guide.py --check` ("GUIDE.html renders from README.md (14 sections,
  30 KB)"), `python -I scripts/changelog.py --check` and `uvx zizmor .github` (no findings): green.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: YYYY-MM-DD`
