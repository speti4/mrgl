---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 028 — Lists as discovery: a shareable list format and a Forever starter list in the repo

## Context

The export format of 015 as a shareable list file (sources and versions, no local paths), and a
"Forever starter list" in the repo that `mrgl import` installs with one command, instead of a search
backend. Discovery without a catalog service: a list a friend sends, or the repo's own, is how a new
Forever player gets the usual addons on day one.
Done when: the starter list installs into an empty fixture flavor with `--dry-run`, and a list with a local
path is refused by the format check.

Decided 2026-10-22: the starter list lives at `lists/forever-starter.json` in the repo (GitHub entries
only, the addons the roadmap's M1 names and the ones with a public Forever release), with a README line
on how to import it; the format check is part of `import` and names the offending entry.

## Docs that move

- `README.md` ("What it will do"): the starter list and the one import command; `docs/development.md`
  ("Layout"): the `lists/` folder and the format rule.

## Not in this change

- Importing the listed version (057, on the install-path branch); entries of other sources (056); a
  search or catalog of any kind.

## Steps

1. [ ] The format check in `list_file`: a local path, an absolute path or a zip entry is refused with the
       entry named; the export of 015 writes only what the format allows
2. [ ] `lists/forever-starter.json` with the GitHub repositories that publish a Forever build today
       (checked against the release metadata, the check documented in the Log, no network in tests)
3. [ ] Tests: the starter list into an empty fixture flavor with `--dry-run` against the stub; a list
       with a local path refused
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
