---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 010 — Update all and update one in the window, with progress and the voice

## Context

An "update all" button per flavor and an update action on each row, progress per addon, a summary in the
voice's two lines, cancel; the row menu items and the per-row progress slot the sketch (017) left disabled
are wired here, and the row menu carries pin, unpin, ignore, unignore and the channel through the engine's
five mark calls (021); the made-up data gains the `channel` field. The rows' "update available" takes the
answer of `update::check` (006, with the version order of 068) instead of the cached-release compare of
009, and a Forever flavor without an AddOns folder says the client has not been started yet. This is the one
click that replaces the CurseForge app's main job.
Done when: a flavor with several updates completes with a correct summary and a cancel leaves no half-written
addon.

Decided 2026-10-22: starts from the tip of task 068's branch, whose per-addon progress callback
(`update::CheckProgress`) and states (`installed-newer`, `rate-limited`, the `BudgetStop` line) the window
shows; the follow-up of 068 about the window's own version compare is part of this task.

## Docs that move

- `docs/development.md`: the window's IPC surface (`update_all`, the per-row update, the mark calls, the
  progress events) and the cancel rule.
- `README.md` ("What it will do"): the window's update, if the bullet still says "task 010".

## Not in this change

- The murloc icon and the release (012); the toolbox views of the window (074); a token prompt in the
  window (the token comes from the environment, 042); the other sources (056).

## Steps

1. [ ] `mrgl-app`: `update_all` and `update_one` commands over the install helper of 055 and
       `update::check` with the progress callback of 068; progress per addon as events to the web view;
       cancel between addons (an addon in flight finishes or is rolled back by the engine's journal, never
       left half-written)
2. [ ] The row menu: pin, unpin, ignore, unignore, channel through the engine's five mark calls (021); the
       per-row progress slot and the row menu items of the sketch (017) enabled; `mock-data.js` and `data.js`
       keep one contract, the made-up data gains `channel`
3. [ ] The rows' "update available" from `update::check`'s answer (its version order), not the cached
       compare of 009; a Forever flavor without an AddOns folder says the client has not been started yet
4. [ ] The summary in the voice's two lines (kind → status bar or toast as 009 routes it); the budget stop
       line of 068 shown once
5. [ ] Tests: Rust tests of the plain functions in `src/list.rs` and the command module over fixture flavors
       (several updates, a cancel, the summary), Node tests under `tests/ui/` for the menu and the progress
       slot
6. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), cut
  from the tip of task 068's branch

## Deviations from the plan

- none yet

## Verification

- (dated, concrete: what was run, what it showed; the window on the real install is the maintainer's manual
  round, 045 and 008)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
