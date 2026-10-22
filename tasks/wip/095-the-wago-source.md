---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 095 — The Wago source

## Context

`wago:<id>` resolves through the Wago Addons external API with the user's Bearer key (`MRGL_WAGO_KEY`,
093), the game version per flavor (`retail`, `classic`, `forever`, the progressing classic by its build), the
three recent releases mapped onto the channels, the download fetched with the same key and sent to Wago's
host only, the rate-limit headers feeding the request budget, the answer cached under `cache\wago\`, a 401
naming the key; the Forever signal is `yes` or `no` from a release for `forever`, and the keyless public data
endpoints (the game list, the slugs per game version) answer the signal and the id lookup without a key.
Decided 2026-10-22: the keyless data endpoints are used as they are; the answers are cached like GitHub's.
Why: the one source after GitHub whose API names the Forever flavor. Done when: `add wago:<id>` installs from
the stub for Forever and retail, a 401 stub names the key, the channels are tested over the three stabilities,
the check stops before the stub's rate limit, a list entry imports, and fixtures classify Forever `yes` and `no`.

Decided 2026-10-22: runs after 094 on the same branch; only the user's own key (D-002, 093).

## Docs that move

- `docs/development.md`: the sources layout (the Wago module, its cache folder, its fixtures), the keys section
- `README.md`: the source list, the `wago:<id>` spec and the `MRGL_WAGO_KEY` line in the usage text

## Not in this change

- The CurseForge source (096); the adopt report (097); the window's key field
- A per-source rate-limit stop of the update check (101)

## Steps

1. [ ] `sources/wago.rs`: resolve by id with the Bearer key from the keys module of 093, the game version
       per flavor (the progressing classic by its build as the GitHub flavor keys do), the three stabilities
       onto the channels, the interface from the supported patches, the cache under `cache\wago\`
2. [ ] The download fetched with the same key, sent to Wago's host only (a test); a 401 names the key; the
       rate-limit headers feed the request budget
3. [ ] The Forever signal `yes` or `no` from a release for `forever`; the keyless data endpoints answer the
       signal and the id lookup without a key
4. [ ] The source joins the router of 092; the list import and the update check take a `wago:` entry
5. [ ] Fixtures (the Forever and the retail answer of one id through the stub's query mapping, a 401, a
       rate-limit answer, the zip behind the download link) and the tests of the done-when lines

## Log

- 2026-10-22 — promoted; runs after 094 on the same branch

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
