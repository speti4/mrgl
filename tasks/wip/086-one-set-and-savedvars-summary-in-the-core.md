---
size: S
origin: done/074-window-toolbox-sets-and-savedvariables.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M3 Toolbox
changelog: housekeeping
---

# 086 — One summary line of `set apply` and `savedvars` in the core, shared by the command line and the window

## Context

The summary lines of `set apply` and of the SavedVariables listing and cleanup move into the core (`sets` and
`savedvars`), and the command line and the window's toolbox (074) both print them from there; the orphan
count agrees with its verb (`1 orphan (…) has no addon folder`, `2 orphans … have`).
Why: 074 copied the command line's summary text into the window, two copies will drift apart, and both say
`1 orphan … have`.
Done when: one function per summary in the core with both callers, the CLI and the window tests assert the
same lines, and the one-orphan line reads `has`.

Decided 2026-10-22: runs first on a branch shared with 085, so the window's new toolbox actions build on the
shared summaries.

## Docs that move

- `docs/development.md`: where the toolbox's summary lines live (the core), named in the voice or the toolbox
  section

## Not in this change

- New toolbox actions in the window (085); any other line the window copied from the command line

## Steps

1. [ ] `crates/mrgl-core/src/sets.rs` and `savedvars.rs`: one function per summary (the apply summary, the
       SavedVariables listing summary, the cleanup summary) that returns the voice line, with the orphan
       count's verb agreeing with the number
2. [ ] `crates/mrgl-cli/src/main.rs` and `crates/mrgl-app/src/toolbox.rs` print the core's lines, the copies gone
3. [ ] Tests: the core's summaries with one and with two orphans; the CLI and the window tests assert the same
       lines (`tests/cli.rs`, `crates/mrgl-app/tests/toolbox.rs`, the UI's Node tests if the text is checked)
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
