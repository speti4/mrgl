---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 097 — Adopt, the report: what source each unmanaged addon of a folder has

## Context

`mrgl adopt --flavor <id>`, read-only: per unmanaged addon the sources its TOCs name (`X-Wago-ID`,
`X-WoWI-ID`, `X-Curse-Project-ID`, `X-Website` on github.com, then the curated GitHub mapping compiled in
from `data/github-sources.json`, keyed by folder name, checked by a test, kept by pull requests), each source
asked whether it publishes a build for the flavor, and a summary: how many addons can be adopted without
CurseForge, how many only with it, how many have no known source; a `.curseclient` marker is reported;
`--json` as every command. The first half of the former 026, after 094 and 095. Decided 2026-10-22: a
CurseForge-only addon is reported here and never written by 098 before the key exists; a source without its
key reads "needs <KEY NAME>". Why: useful before any write and any key; it measures the roadmap's M4 goal.
Done when: a fixture AddOns folder with every header shape (each id header, a GitHub and a non-GitHub website,
mapping only, no hint, a multi-folder addon, an unsubstituted version) reports the sources offline against the stubs.

Decided 2026-10-22: the mapping file is compiled in with `include_str!` like `data/forever-notes.json`,
keyed by folder name, with a schema test, seeded from `lists/forever-starter.json`; fingerprint matching is
not in M4.

## Docs that move

- `docs/development.md`: the adopt module, the mapping file and its test, the TOC parser's website field
- `README.md`: the `adopt` command in the usage text
- `CONTRIBUTING.md`: how an entry joins `data/github-sources.json`, if the Forever notes have such a paragraph

## Not in this change

- Any write (098 `--apply`); the CurseForge source itself (096); fingerprint matching (not in M4)
- The window's adopt view (a follow-up once the report exists)

## Steps

1. [ ] `toc.rs`: a typed `X-Website` with the github.com reader, the id shapes of the three id headers, an
       unsubstituted packager token (`@project-version@`) read as no version
2. [ ] `data/github-sources.json` compiled in, keyed by folder name, a schema test, seeded from the starter list
3. [ ] `adopt.rs`: the unmanaged addons of a flavor, their sources in the order of the headers then the mapping,
       each source asked for a build of the flavor (a keyless source reads "needs <KEY NAME>"), the three counts,
       the `.curseclient` marker
4. [ ] `main.rs`: `mrgl adopt --flavor <id>` with `--json`, through the voice
5. [ ] A fixture game folder under `tests/fixtures/adopt/` (with and without the marker) with every header shape,
       and the tests of the done-when line against the stubs of 094 and 095

## Log

- 2026-10-22 — promoted; runs on the branch of 094 and 095, 098 follows on the same branch

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
