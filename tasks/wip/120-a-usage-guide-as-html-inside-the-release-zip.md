---
size: M
origin: session 2026-10-24
created: 2026-10-24
started: 2026-10-24
plan: none
milestone: M5 Trust
---

# 120 — A usage guide as HTML inside the release zip

## Context

The release zip carries a usage guide, `GUIDE.html`, one self-contained page generated from the README's user
sections (what it does, install, the window, the command line and its commands, the sources and the keys,
backups and sets, where the data lives) by a script under `scripts/` that `release.ps1` runs, in the voice of
the project and offline, no external asset; the README stays the source of the words.
Why: the M5 Trust roadmap line, and a user who unpacks the zip has no README next to the exes.
Done when: `release.ps1` packs `GUIDE.html` next to the exes, the page opens offline and reads as the README's
user half, a CI check keeps the generated page in step with the README, and the docs name the script.

Decided 2026-10-24: filed and started in one step from the roadmap's M5 line; the README stays the one source
of the user text, the page is generated, never hand-edited.

## Docs that move

- `docs/development.md`: the Layout bullet of `scripts/`, the release paragraph (what the zip holds), the
  sentence that says a usage guide is M5
- `README.md`: the Install section names the guide inside the zip; any README restructuring the generator
  needs (section markers) stays invisible to a reader
- `CONTRIBUTING.md`: the generated-page check in "Checks before a commit", if a check is added

## Not in this change

- Signed exes (103) and self-update (031)
- A guide for the window's every control (the page mirrors the README, no more)
- Screenshots or any binary asset

## Steps

1. [ ] Choose the generator: a Python script under `scripts/` (the repo's scripts are Python and PowerShell,
       D-002 asks for nothing to install) that reads the README's user sections between markers or by
       heading and writes one HTML file with inline CSS, no script, no external asset
2. [ ] `scripts/release.ps1` runs it into the stage next to the exes; `docs/development.md` and the README
       name it
3. [ ] A check that fails when the generated page is out of step with the README, wired where the other
       generated-file checks run (the changelog check is the model), locally and in CI
4. [ ] Tests of the generator (its Python tests or a fixture README) as the repo tests its other scripts
5. [ ] Verification: `release.ps1` on this tip packs the page, the page opens offline in a browser

## Log

- 2026-10-24 — filed and promoted

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
