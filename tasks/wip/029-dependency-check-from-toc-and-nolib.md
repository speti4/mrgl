---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 029 — Dependency check from the TOC `## Dependencies` and `## RequiredDeps` and the `release.json` `nolib` flag

## Context

Read `## Dependencies` and `## RequiredDeps` from each installed addon's TOC and the `nolib` flag of the
source's `release.json`, and report a missing required dependency after an install and in `list`; an
embedded library (a `nolib` build not chosen) is not missing. A missing library is the second most common
"it does not load" after a wrong flavor pick.
Done when: fixture addons with met, missing and embedded dependencies report as documented, offline.

Decided 2026-10-21: the TOC parser already collects the required and optional dependency names (002);
this task adds the check over a scan: a required name is met when a folder of that name is in the scan
or is a subfolder of the addon itself (an embedded library, which is what a non-`nolib` build carries),
missing otherwise; an optional dependency is never reported; the pick's `nolib` flag (003) is recorded
on the sidecar entry so the check knows an embedded library is expected; the report is a warning line
per addon with the missing names, in `list` and after an install, never a refusal.

## Docs that move

- `docs/development.md` ("Layout"): the dependency check and what met, missing and embedded mean.
- `README.md`: the missing-dependency line a user may see.

## Not in this change

- Installing a missing dependency from a source (a later task, once the sources know library repos);
  the window's row (010); optional dependencies.

## Steps

1. [ ] The check over a scan: met, missing and embedded per addon from the TOC's required names and the
       addon's own subfolders
2. [ ] The sidecar entry records the pick's `nolib` flag (optional in the JSON; absent means unknown)
3. [ ] `mrgl-cli`: the warning line in `list` and after `add` (plain through the voice, and `--json`
       fields per addon)
4. [ ] Tests: fixture addons with a met, a missing and an embedded dependency, offline
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
