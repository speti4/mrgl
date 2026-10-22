---
size: S
origin: done/095-the-wago-source.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 106 — The Wago update check asks for its addons in bulk, one request per game version

## Context

The update check gathers the Wago entries per game version and asks `POST /addons/_recents` once
for each, through the `Http::post` of 092, instead of one request per addon; the cache and the per-addon
answer shape stay the same, and a single `add` or `update <name>` still asks for the one addon.
Why: one request per addon against a limit of 60 runs out on a folder of a few dozen Wago addons; the
bulk endpoint makes the check one request per game version.
Done when: a fixture check over several Wago entries makes one stubbed POST per game version, the answers
land in the same cache files, and the budget test of 095 still stops before the limit.

Decided 2026-10-22: the second of four source follow-ups on one branch (101, 106, 104, 105), after 101.

## Docs that move

- `docs/development.md`: the Wago source's bulk check and its stub (a POST answer file)

## Not in this change

- A bulk check for the other sources; the per-source stop itself (101)

## Steps

1. [ ] `sources/wago.rs`: a bulk lookup of several ids per game version through `Http::post`, parsed into
       the same per-addon answer and written into the same cache files
2. [ ] `update.rs`: the check gathers the Wago entries per game version and asks once; `add` and
       `update <name>` keep the single request
3. [ ] Tests: a stubbed POST answer per game version, one request per version over several entries, the
       cache files, the budget stop of 095 unchanged

## Log

- 2026-10-22 — promoted

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
