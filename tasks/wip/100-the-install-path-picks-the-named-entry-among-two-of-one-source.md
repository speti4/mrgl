---
size: S
origin: done/089-update-one-reports-a-second-entry-of-the-same-source.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 100 — The install path picks the named entry among two sidecar entries of one source

## Context

When two sidecar entries carry the same source, the install path and `update <name>` work on the
entry of the addon that was named, not on whichever entry's name sorts first; `managed_entry` looks up by
source and name together where a name is known.
Why: with a renamed addon next to its old name, `update <name>` can update the other entry today, since
`managed_entry` finds the first entry of the source alone.
Done when: a fixture with two entries from one source, the old name sorting first, updates the named
entry under `update <name>` and leaves the other alone, in the text and the JSON.

Decided 2026-10-22: the second of three follow-ups on one branch (099, 100, 102), in id order.

## Docs that move

- `docs/development.md`: the install engine's entry lookup, if it describes it

## Not in this change

- Merging or removing duplicate entries; the window's reporting of the second entry (099)

## Steps

1. [ ] `install.rs`: `managed_entry` looks up by source and name where a name is known, by source alone
       otherwise; the callers of the one install path hand the name through
2. [ ] Tests: a fixture with two entries from one source, the old name sorting first; `update <name>` updates
       the named entry and leaves the other alone, text and JSON; `update --all` unchanged

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
