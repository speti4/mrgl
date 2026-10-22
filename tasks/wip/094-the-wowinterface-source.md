---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 094 — The WoWInterface source

## Context

`wowi:<id>` resolves through the MMOUI API (`filedetails/<id>.json`, no key), one file per project; the
flavor comes from the project's compatibility list (a metadata pick for retail and the classic flavors, a
guess without one and for Forever), a pending file is reported and not installed, the download is checked
against the API's MD5, the answer is cached under `cache\wowi\` (the API allows an hour), and the Forever
signal stays `unknown` until WoWInterface lists a Forever compatibility. Runs after 092 and 093, before 095.
Decided 2026-10-22: the first new source, since it works without a key; the interface check after install
is the safety net of a Forever guess. Why: the one keyless source after GitHub, and the proof of the router.
Done when: `add wowi:<id>` installs from the stub, a compatibility list picks by metadata and none is a
guess, a pending file is reported, a wrong MD5 is refused before the zip opens, a list entry imports, and a
fixture answer classifies the Forever build as `unknown`.

Decided 2026-10-22: the Forever signal is `unknown` wherever the API does not name Forever, never inferred
from a classic or retail compatibility; the WoWInterface answers are cached like GitHub's.

## Docs that move

- `docs/development.md`: the sources layout (the WoWInterface module, its cache folder, its fixtures)
- `README.md`: the source list and the `wowi:<id>` spec in the usage text
- `Cargo.toml`: the one-line reason if an MD5 crate is added (else a hand-rolled MD5 with test vectors)

## Not in this change

- The Wago source (095) and the CurseForge source (096); the adopt report (097)
- A Forever signal inferred from a non-Forever compatibility

## Steps

1. [ ] `sources/wowinterface.rs`: resolve by id through `filedetails/<id>.json`, the flavor from the
       compatibility list (metadata pick for retail and the classic flavors, a guess for Forever and without
       a list), a pending file reported and not installed, the channels, the cache under `cache\wowi\`
2. [ ] The download checked against the API's MD5 before the engine opens the zip; the Forever signal `unknown`
3. [ ] The source joins the router of 092 (`Sources::from_env`, `Sources::cached`); the list import and the
       update check take a `wowi:` entry
4. [ ] Fixtures under `tests/fixtures/sources/` (an answer with a compatibility list, one without, a pending
       file, a wrong MD5, the zip behind the download URL with the stub's query mapping) and the tests of the
       done-when lines

## Log

- 2026-10-22 — promoted; runs on the branch of 092 and 093, 095 follows on the same branch

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
