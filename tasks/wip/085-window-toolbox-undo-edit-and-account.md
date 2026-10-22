---
size: M
origin: done/074-window-toolbox-sets-and-savedvariables.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 085 — The window's toolbox undoes an apply, edits a set and names the account

## Context

The toolbox tab (074) offers the undo of the last apply (`sets::undo_apply`), the enable, disable and drop of
one addon in a stored set (`sets::edit_set`), and passes an account through `sets::apply_set_in_account` when
one realm and name sit under two accounts (073), each refused while the client runs as the apply is.
Why: 073 landed the undo, the edit and `--account` on the command line in the same wave as the window's set
switch, so the window applies sets but cannot take one back or change one.
Done when: on the fixture flavor the window undoes an apply byte for byte, an edit changes only the named
addon in `sets.json`, and the two-account case applies to the chosen account.

Decided 2026-10-22: runs after 086 on the same branch, on top of the shared summary lines.

## Docs that move

- `docs/development.md`: the UI section (the toolbox's undo, edit and account controls, `toolbox.js`) and the
  IPC commands (the new toolbox commands and their answers)
- `README.md`: the window's feature list and the usage line of the window, where the toolbox is described

## Not in this change

- Saving a set from the window; the `WTF` backup and restore in the window; a new core function (073 has them)

## Steps

1. [ ] `crates/mrgl-app/src/toolbox.rs` and `commands.rs`: the undo (`sets::undo_apply`), the edit
       (`sets::edit_set`: enable, disable, drop) and the apply with an account (`sets::apply_set_in_account`),
       each behind the running-client guard and the one-write-at-a-time rule of 074, every message through
       the voice
2. [ ] `ui/`: the undo button with what it restores, the edit controls on the set's chips, the account pick
       when a character sits under two accounts; the pure part in `ui/toolbox.js` with Node tests, the
       mock data with the two-account case
3. [ ] Tests in `crates/mrgl-app/tests/toolbox.rs`: the undo restores byte for byte, an edit changes only the
       named addon in `sets.json`, the two-account case applies to the chosen account, each refused with the
       stand-in client running
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
