---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M2 Window MVP
---

# 011 — Forever notes dataset: dead, crippled or viable, from the research

## Context

Two signals on an addon's row in the window and in `mrgl list`: derived, whether the addon's source
publishes a `forever` build (`release.json`: yes, no or unknown); and curated, a small JSON data file in
the repo for the crippled cases only (a build exists, but the hidden combat values or the missing combat
log leave it half-working), one line per addon with a reason, a `Verified:` date and a source, extended
through pull requests. Forever hides combat values and removes the combat log; a user should see why an
addon will not work before installing it.
Done when: the derived signal covers every fixture `release.json`, the curated file carries the research
page's crippled addons with dates, and a stale entry is visible from its date.

Decided 2026-10-21: the Forever status is derived first, curated second. The derived signal is read from
the release list the source already fetches (a `release.json` with a `forever` flavor: yes; one without:
no; no `release.json` or a pick by name: unknown) and shown by `add --dry-run` and, for a managed addon,
by `list`; the curated file is `data/forever-notes.json` (one entry per addon keyed by the source spec:
reason, `verified` date, source URL), read at build time into the core, and an entry older than a
documented age is marked stale in the output. The window's data contract in `mock-data.js` (`forever:
{ build, note }`) already names the two signals and does not change.

## Docs that move

- `docs/development.md` ("Layout", "Fixtures"): the `data/` file, its schema and how to add an entry.
- `CONTRIBUTING.md`: a line on contributing a Forever note (reason, date, source) through a pull request.
- `README.md`: what the Forever note on a row means.

## Not in this change

- The window's row (017's sketch shows the made-up data until the shell binds the core, 009); a signal
  for other flavors; automatic staleness checks in CI.

## Steps

1. [ ] The derived signal in the source: `forever` build yes, no or unknown from the release list, on the
       pick and for a managed addon's source
2. [ ] `data/forever-notes.json` with the research page's crippled addons (reason, `verified`, source) and
       a core reader with the stale rule and a schema check test
3. [ ] `mrgl-cli`: the note and the build signal on `add --dry-run` and in `list` (plain and `--json`),
       through the voice
4. [ ] Tests: every fixture `release.json` classified, the curated file parsed, a stale entry marked
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
