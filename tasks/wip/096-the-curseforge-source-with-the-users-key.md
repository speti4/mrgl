---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 096 — The CurseForge source with the user's key

## Context

`curse:<id>` resolves through the CurseForge Core API with the user's key (`MRGL_CURSEFORGE_KEY`, 093):
the mod and its files per game version type (Forever included), the release types mapped onto the channels,
the download carrying the key too; a mod with distribution off fails with a line naming its page, and a zip
in hand installs with the source recorded; no API answer is cached, as the terms require, so the Forever
signal in `list` comes from what the sidecar recorded at install, else `unknown`. The code and the stubs land
before the key; the manual round and the type ids wait for it. Decided 2026-10-22: only the user's own key
(D-002, 093); never a cache of CurseForge answers. Why: the retail and classic folders are mostly CurseForge.
Done when: `add curse:<id>` installs from the stub with a key and names the key without one, a
distribution-off mod names its page and installs from a zip, a test asserts the key reaches the two CurseForge
hosts only and never a cache or the sidecar, a list entry imports, and a fixture classifies the Forever build.

Decided 2026-10-22: the code and the stubs are built now, on the branch of the source follow-ups (101 to
106), without the key; the task stays open after that change lands, until the key of 007 allows the manual
round and the re-read of the game version type ids. Also here: `Source::folders` from a file's `modules` and
the adopt lines (097) with a `curse:` source that can be asked.

## Docs that move

- `docs/development.md`: the sources layout (the CurseForge module, no cache folder, its fixtures), the keys
  section, the sidecar's recorded Forever signal
- `README.md`: the source list, `curse:<id>`, `MRGL_CURSEFORGE_KEY`, the distribution-off line

## Not in this change

- The manual round and the confirmation of the game version type ids (they wait for the key of 007)
- The window's key field; fingerprint matching

## Steps

1. [ ] `sources/curseforge.rs`: the mod and its files per game version type with the key from the keys module,
       the release types onto the channels, the download with the key, the key sent to the two CurseForge hosts
       only (a test), no cache of any answer, `listed_versions` none
2. [ ] A mod with distribution off fails with a line naming its page; a zip in hand installs with the source
       recorded; the sidecar records the Forever signal at install (an optional field, the schema unchanged)
       and `list` reads it, else `unknown`; the window's last-check file keeps no `curse:` version
3. [ ] The source joins the router; the list import and the update check take a `curse:` entry; `Source::folders`
       from the file's `modules`; the adopt lines with an askable `curse:` source
4. [ ] Fixtures (the mod, the files per type for Forever and retail, a distribution-off mod, the zip behind the
       CDN URL, a 401) and the tests of the done-when lines; the type ids as constants with a note that they
       are re-read once a key exists

## Log

- 2026-10-22 — promoted; the code and the stubs now, the manual round when the key arrives

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
