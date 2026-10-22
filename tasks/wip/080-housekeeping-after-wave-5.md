---
size: S
origin: done/064-blizzard-addons-are-not-missing-dependencies.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 080 — Housekeeping after the M1 fixes: one `Blizzard_` prefix, the doctor's formatter alias, the voice sample in the docs

## Context

`dependencies::CLIENT_PREFIX` and `savedvars::CLIENT_PREFIX` become one shared constant and helper;
the doctor calls `zip_entry::size_text` directly and the `bytes_text` alias import goes; and the planned voice
sample in `README.md` and `docs/nerglish.md` shows the per-addon progress lines of the update check (068) as
the command line and the window (010) print them. Two copies of the `Blizzard_` rule will drift apart (064,
071), the alias was a bridge for the parallel branches of wave 5, and the sample predates the progress lines.
Done when: one constant with both callers, no alias import, the sample matches a real `update --all` run's
lines, and every existing test is unchanged.

Decided 2026-10-22: second of the chain 076 → 080 on one branch, in wave 6.

## Docs that move

- `README.md` and `docs/nerglish.md`: the voice sample shows the progress lines of `update --all`.
- `docs/development.md`: the module that holds the shared `Blizzard_` constant, where the rule is named.

## Not in this change

- Any change to the dependency or SavedVariables rules themselves (076 changes the dependency rule); the
  window's output.

## Steps

1. [ ] One shared `Blizzard_` constant and helper in the core, called by `dependencies` and `savedvars`
2. [ ] `doctor.rs` calls `zip_entry::size_text` directly, the `bytes_text` alias import goes
3. [ ] The voice sample in `README.md` and `docs/nerglish.md` matches the lines of a real `update --all` run
4. [ ] Every existing test unchanged and green; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 076 → 080

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
