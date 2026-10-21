---
size: S
origin: done/015-list-export-import-and-interface-warning.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 057 — An import option that installs the listed version, not the current release

## Context

`import --exact` (or the name the CLI's flags prefer) installs the version the list file records
for each entry, through the source's release list (the tag of that version), and reports an entry whose
version the source no longer offers; the default stays the current release. A list exported from a
working folder is a known-good set; a second machine wants that set, not whatever shipped since.
Done when: a fixture list with an older version installs that version from the stub, and a version the
stub lacks is reported, not installed.

Decided 2026-10-22: third in the chain 055 → 006 → 057; the exact install goes through the one install
helper of 055 with a resolve of a named version instead of the latest pick.

## Docs that move

- `README.md` ("What it will do") and `docs/development.md`: the import flag and what a missing version
  reports.

## Not in this change

- Entries of other sources (056); a list file that names a local path (028); the shareable list format
  itself (028).

## Steps

1. [ ] The source resolves a named version (the release tagged with it) next to the latest pick
2. [ ] `import --exact` through the install helper of 055; a version the source lacks is reported, the
       entry skipped, the summary counts it
3. [ ] Tests: a fixture list with an older version installs it from the stub; a version the stub lacks is
       reported; the default import unchanged
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), third
  of the chain 055 → 006 → 057

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
