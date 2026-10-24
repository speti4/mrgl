---
size: S
origin: wip/096-the-curseforge-source-with-the-users-key.md
created: 2026-10-22
started: 2026-10-24
plan: none
milestone: M4 Sources
---

# 115 — A distribution-off CurseForge entry is a state of its own in the update check

## Context

A `curse:` entry whose project allows no download outside CurseForge (`allowModDistribution: false`) gets its
own state in the update check and the window's row, as the pending state of 104, counted apart from the
failed checks: the line names the page and the zip-in-hand way (`add <zip> --as curse:<id>`), and a run with
such entries only ends with exit code 0.
Why: 096 reports it as a failed check, which turns every run red for a choice the author made, not an error.
Done when: the distribution-off fixture (id 4244) ends an `update --all` with exit code 0 and its own line and
JSON state, the window's row shows it, and a failed check still ends with 1.

Decided 2026-10-24: the second of three follow-ups on one branch (114, 115, 116).

## Docs that move

- `docs/development.md`: the update check's states (text and JSON) and the window's row states
- `README.md`: the CurseForge paragraph's distribution-off line, if it says the check fails

## Not in this change

- The 096 task file (another session's open task; its recorded deviation on this state stays as written)
- The window's own zip-in-hand install with a source (116)

## Steps

1. [ ] `crates/mrgl-core/src/update.rs` (and the source's error): `Error::DistributionOff` becomes a state of
       its own in the check's outcome, counted apart from the failed checks, with the line that names the
       page and `mrgl add <file.zip> --as curse:<id>`
2. [ ] `crates/mrgl-cli/src/main.rs`: the text line through the voice, the JSON state and count, exit code 0
       for a run whose only non-success is this state; a failed check still ends with 1
3. [ ] `crates/mrgl-app`: the window's row shows the state (as the pending state of 104), the summary counts
       it; the offline check's last answers keep the state
4. [ ] Tests: the 4244 fixture under `update --all` (text, JSON, exit code), a failed check next to it with
       exit 1, the window's row, the Node tests of the UI modules

## Log

- 2026-10-24 — promoted

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
