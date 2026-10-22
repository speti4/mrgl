---
size: S
origin: done/094-the-wowinterface-source.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 104 — A WoWInterface file waiting for approval is a state of its own in the update check

## Context

A WoWInterface project whose newest file waits for the site's approval (`UIPending`) gets its own
state in the update check and the window's row (a pending release, nothing to do yet), counted apart from
the failed checks, so `update --all` ends with exit code 0 while the site approves it.
Why: 094 reports a pending file as a failed check, which turns a daily run red for a day or two that is the
site's, not the user's.
Done when: the pending fixture (id 1003) ends an `update --all` with exit code 0 and its own line and JSON
state, the window's row shows it, and a failed check still ends with 1.

Decided 2026-10-22: the third of four source follow-ups on one branch (101, 106, 104, 105), after 106.

## Docs that move

- `docs/development.md`: the update states (the pending release), the window's state list

## Not in this change

- Installing the previous, approved file instead (the API offers one file per project)

## Steps

1. [ ] `update.rs`: a pending release is its own state (`Error::ReleasePending` of 094 maps onto it), counted
       apart from the failed checks, exit code 0 for a run with pending entries only
2. [ ] The text (through the voice), the JSON state and the window's row and state list carry it
3. [ ] Tests: the pending fixture under `update --all` (exit 0, the line, the JSON), a window row, a failed
       check still exits 1

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
