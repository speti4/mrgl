---
size: S
origin: done/082-install-and-update-report-the-interface-check.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 089 — `update <name>` says what happened when another entry of the same source already has the pick's version

## Context

When the one install path finds a second sidecar entry from the same source at the pick's version
and answers "installed already", `update <name>` reports that entry and the outcome instead of counting
the addon as updated and printing no line; the JSON carries the same.
Why: two sidecar entries from one source (a renamed addon next to its old name) make a silent, miscounted
update today.
Done when: a fixture with two entries from one source prints the line and counts the addon right, in the
text and the JSON, and `update --all` agrees.

Decided 2026-10-22: the second of three wave-7 follow-ups on one branch (088, 089, 090), in id order.

## Docs that move

- `docs/development.md`: the `update` command's outcomes, if it lists them

## Not in this change

- Merging or removing the duplicate entries (the line names them; a cleanup is a task of its own if wanted)

## Steps

1. [ ] The install path's "installed already" answer carries the entry it found; `update <name>` and
       `update --all` report it as its own outcome, in the text (through the voice) and the JSON, with the
       counts right
2. [ ] Tests: a fixture with two entries from one source, `update <name>` and `update --all`, text and JSON

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
