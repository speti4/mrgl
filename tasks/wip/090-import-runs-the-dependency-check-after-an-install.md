---
size: S
origin: done/082-install-and-update-report-the-interface-check.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 090 — `import` runs the dependency check after an install, as `add` and `update` do

## Context

`mrgl import` reports the missing dependencies of every addon it installed, through the same
`Engine::check_installed` the install summary of `add` and `update` reads (079, 082), in the text and the
JSON next to the interface warnings it already prints.
Why: an imported list is the one path that installs many addons at once, and the one that says nothing
when a required library is not on the list.
Done when: a fixture list that installs an addon with a missing dependency prints the warning and carries
it in the JSON, and a complete list prints none.

Decided 2026-10-22: the third of three wave-7 follow-ups on one branch (088, 089, 090), in id order.

## Docs that move

- `docs/development.md`: the `import` command's summary, the dependency warning
- `README.md`: the import usage, if it describes what the summary reports

## Not in this change

- Installing a missing dependency on its own (the warning names it, as `add` does)

## Steps

1. [ ] `main.rs`: `import` runs `Engine::check_installed` over what it installed and prints the missing
       dependencies next to the interface warnings, in the text and the JSON
2. [ ] Tests: a fixture list with a missing dependency prints the warning and carries it in the JSON; a
       complete list prints none

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
