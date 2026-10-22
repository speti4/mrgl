---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 092 — One source per spec: the router, the spec forms and the generic paths

## Context

`SourceSpec` gains `wago:<id>`, `wowi:<id>` and `curse:<id>` (and the addon page URLs an id can be read
from), a `Sources` value implements `Source` by dispatching on the spec and replaces every direct `GitHub`
construction in the command line and the window; the Forever build signal moves onto the trait (default
`unknown`); the list import and the update check ask any configured source, and an entry of a source this
build lacks reads "source not configured", not "not importable"; the test stub maps a query string to a
file and `Http` gains `post`. The first task of M4: the source tasks 094 to 096 plug in here.
Why: every caller is GitHub-only today, so each new source would touch the same eight places.
Done when: every existing test passes unchanged, `add wago:x` without the Wago source names what is missing,
a `wowi:` list entry reads "source not configured", and the stub serves two flavors of one URL from two files.

Decided 2026-10-22: the first task of M4, before the key store (093) and the sources (094 to 096); the
Forever signal is `unknown` wherever a source does not name Forever, never inferred.

## Docs that move

- `docs/development.md`: the sources layout (the router, the spec forms), the fixture stub's query mapping
- `README.md`: the source spec forms the commands accept, if the usage text lists them

## Not in this change

- Any real second source (094 WoWInterface, 095 Wago, 096 CurseForge) and the key store (093)
- The adopt report and its TOC header reading (097, 098)

## Steps

1. [ ] `sources.rs`: the three new `SourceSpec` variants, their parse forms (spec and page URL) and canonical
       display; the Forever signal on the `Source` trait with the `unknown` default
2. [ ] A `Sources` router value built once from the environment and used by every CLI and window call site
       (`main.rs`, `commands.rs`, `list.rs`, the offline view of 078 per source)
3. [ ] `list_file.rs` and `update.rs`: a configured source is asked; a source this build lacks reads
       "source not configured"; `install.rs` name hints per spec
4. [ ] The folder HTTP stub maps a query string to a file; `Http` gains `post`, stubbed the same way
5. [ ] Tests: the done-when lines; every existing test passes unchanged

## Log

- 2026-10-22 — promoted; runs on its own branch, 093 follows on the same branch

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
