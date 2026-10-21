---
size: S
origin: done/011-forever-notes-dataset.md
created: 2026-10-21
started: 2026-10-22
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

1. [ ] The research: the community sources on Forever's combat restrictions and the addons they name,
       each with its URL and the day read; the research page under `docs/research/`
2. [ ] One `addons` entry per confirmed addon in `data/forever-notes.json` (canonical `github:` spec,
       kind, one-line reason, `verified`, `source`); the schema test green
3. [ ] A CLI test: `list` on a fixture Forever folder with one of them managed shows the note line
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, at close)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
