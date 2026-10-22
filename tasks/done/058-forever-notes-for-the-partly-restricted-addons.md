---
size: S
origin: done/011-forever-notes-dataset.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 058 — Forever notes for the addons the community lists as partly restricted

## Context

The research page gains the addons a community source names as partly restricted on Forever (the
aura and nameplate addons that read combat values), each with the source and the date, and the curated
notes file (011, `data/forever-notes.json`) gains one entry per addon with its kind. The curated file
starts with the restriction kinds and no addon; the first users will ask about exactly these addons.
Done when: the research page lists them with sources, the data file carries them, and `list` on a
Forever folder with one of them installed shows the note.

Decided 2026-10-22: a note enters the data file only with a public source the research page cites
(a developer's statement, a community wiki page, a release note), never from hearsay; an addon the
sources disagree on stays on the research page as an open question and out of the data file.

## Docs that move

- `docs/research/` gains (or extends) the Forever restrictions page with the addons, sources and dates;
  `docs/development.md` ("Forever notes") if the schema or the rule for adding a note changes.

## Not in this change

- A new restriction kind the sources do not justify; the window's note slot binding (009); any
  Blizzard material (D-005).

## Steps

1. [x] The research: the community sources on Forever's combat restrictions and the addons they name,
       each with its URL and the day read; the research page under `docs/research/`
2. [x] One `addons` entry per confirmed addon in `data/forever-notes.json` (canonical `github:` spec,
       kind, one-line reason, `verified`, `source`); the schema test green
3. [x] A CLI test: `list` on a fixture Forever folder with one of them managed shows the note line
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — the research, read on 2026-10-22: Blizzard's Midnight article and the Patch 1.60.1 API notes of
  the community wiki for the restriction kinds; five community guide and status pages for the names they
  list; then every named addon checked at its own repository (the current release's `release.json`, the
  release notes, the README, the Forever issues and who answered them). Blizzard names no addon; the guide
  pages disagree with each other and none is an author's word, so they only supplied names to check.
- 2026-10-22 — seven addons confirmed by their authors, all under the two existing kinds, no new kind:
  Grid2, TellMeWhen and ForeverAuras (`neroxrw`) with `secret-combat-values`, Musca Auras with
  `combat-log-events` (each a Forever build and an author's statement), and Thanks for the Buff,
  SoundAlerter Forever and the `Tulkas22` ForeverAuras fork (the author's statement, but no release zip for
  `add github:` to install). Seven open questions: Plater, Deadly Boss Mods, BigWigs, Details!, ElvUI, Spy and
  Parrot 3; WeakAuras has no Forever build and is not a partly restricted case. Client faults (the weapon
  imbue API, saved variables) are named on the page as faults, not kinds. The page:
  `docs/research/2026-10-22-forever-addon-restrictions.md`.
- 2026-10-22 — `data/forever-notes.json` gains the four installable confirmed addons, keyed by their canonical
  specs, `verified` 2026-10-22, each source the author's statement; the schema test is green.
- 2026-10-22 — `tests/cli.rs`: `list_shows_the_curated_note_of_a_partly_restricted_addon` lists a fixture
  Forever folder with Grid2 managed through a synthetic sidecar entry (`github:michaelnpsp/Grid2`): the line
  ends `forever:unknown,note`, the warning line `forever note on Grid2: …` carries the reason, date and source,
  and the JSON's `forever` object the note, kind and source. The expectation reads the compiled-in note, so a
  later re-check of its date or its turn to stale does not break the test.
- 2026-10-22 — `docs/development.md` ("Forever notes"): a new paragraph on where a note comes from (the
  decided rule) and why an addon without a release zip stays on the research page. `README.md` links the
  research folder, not the pages, so it does not change.

## Deviations from the plan

- Three addons confirmed by their authors stay out of the data file: their repositories publish no release
  zip, so `add github:<owner>/<repo>` cannot install them and a note keyed by their spec would never meet a
  managed addon. They are on the research page with their sources; `docs/development.md` names the rule.
- The schema check compares `verified` with today in UTC: between local midnight and UTC midnight a date of
  the local day reads as in the future. The notes keep the day the sources were read (2026-10-22) and the
  checks ran once the UTC day had turned; the slack in the check is a follow-up.
- `CONTRIBUTING.md` ("Forever notes") still asks only for a source URL, not for the research page; left
  unchanged in this change and named as a follow-up.

## Verification

- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`
  (the core unit tests 141 with the schema test over the four new notes, the CLI end-to-end tests 83 passed and
  1 ignored stand-in), `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check`:
  all green on the task branch, run after 00:00 UTC on 2026-10-22. No manifest changed, so no `cargo deny check`.
- 2026-10-22 — `list_shows_the_curated_note_of_a_partly_restricted_addon` passes: Grid2 managed through a
  synthetic sidecar entry lists as `Grid2  4.1.9  16001  managed  github:michaelnpsp/Grid2
  forever:unknown,note`, followed by `forever note on Grid2: auras are hidden from addons in combat: …
  (verified 2026-10-22, https://github.com/michaelnpsp/Grid2/issues/471)`, and the JSON's `forever` object
  has the note, `kind` `secret-combat-values`, the source and `stale` false.
- 2026-10-22 — every source on the research page was read on the day; each of the four notes links the
  author's statement, and every spec is the canonical one of its repository (GitHub's API names the same
  owner and repository).

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
