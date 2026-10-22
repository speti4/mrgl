---
size: S
origin: done/100-the-install-path-picks-the-named-entry-among-two-of-one-source.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 108 — `update --all` over two entries of one source whose first install takes the second entry's folder

## Context

When a release zip carries the folder of a second sidecar entry of the same source, the first
install takes that folder over and drops the second entry; `update --all` then reports the dropped entry
as folded into the first (its name, the folder, the entry it joined) instead of a "stays at <version>" line
about an entry that no longer exists, in the text and the JSON.
Why: after 100 the line for the second entry describes a sidecar entry the first install already removed.
Done when: a fixture with two entries of one source and a zip carrying both folders reports the second
entry as folded into the first under `update --all`, text and JSON, and the sidecar holds one entry after.

Decided 2026-10-22: the first of two follow-ups on one branch (108, 107).

## Docs that move

- `docs/development.md`: the `update --all` outcomes, if it lists them

## Not in this change

- The window's update over the same case (it reads the core's outcome once the core has one)

## Steps

1. [ ] The install path reports the sidecar entries it folded into the installed one; `update --all` turns
       that into the "folded into" outcome for the dropped entry (through the voice) and the JSON
2. [ ] Tests: a fixture with two entries of one source and a zip with both folders, `update --all` text and
       JSON, the sidecar with one entry after

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
