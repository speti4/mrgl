---
size: S
origin: done/095-the-wago-source.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 105 — A Wago page address resolves to its `wago:<id>` through the keyless slugs list

## Context

`add https://addons.wago.io/addons/<slug>` (and the same address in a list) resolves the slug to the
addon's id through `Wago::id_of_slug` over the keyless slugs endpoint of the flavor's game version, and the
sidecar records `wago:<id>`; a slug the list does not carry says so.
Why: 092 refuses the Wago page address with a hint, since it names a slug, not an id; 095 brought the slugs
list that answers it without a key.
Done when: the page address of a fixture slug installs from the stub as `wago:<id>`, an unknown slug names
the game version it was looked up in, and a list entry with the address imports.

Decided 2026-10-22: the last of four source follow-ups on one branch (101, 106, 104, 105).

## Docs that move

- `docs/development.md`: the spec forms (the Wago page address)
- `README.md`: the accepted addresses in the usage text

## Not in this change

- A CurseForge page address (its slug needs the key, 096)

## Steps

1. [ ] `sources.rs`: the Wago page address parses into a slug form that the router resolves to `wago:<id>`
       through the slugs list of the target's game version before the spec is recorded
2. [ ] `add` and the list import record `wago:<id>`; an unknown slug names the game version it was looked up in
3. [ ] Tests: a fixture slug installs from the stub, an unknown slug's line, a list entry with the address

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
