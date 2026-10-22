---
size: M
origin: done/014-addon-sets-with-enable-and-disable.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 073 — Addon sets: undo an apply, edit a stored set, `--account`, and uninstalled addons on save

## Context

`set apply` keeps the `AddOns.txt` files it replaced so `set undo` puts them back; `set enable|disable|drop
<set> <addon>` edits a stored set without the files; `--account` picks a character when one realm and name
sit under two account folders (027); `set save` warns about addons that are no longer installed. A wrong
apply has no way back today, and a set can only change by saving it again from the files.
Done when: a fixture apply round-trips through undo byte for byte, an edit changes only the named addon, and
the two-account case resolves with `--account`.

Decided 2026-10-22: second of the chain 062 → 073 on one branch, in wave 6. `set save` keeps an addon that
is no longer installed and warns about it (a set is the user's data; `set apply` already writes such an
addon's line with a warning). `sets::apply_set` keeps its signature and its result: the undo record is
written by the apply itself under the flavor's folder of the mrgl home, so a caller of today (the window's
toolbox, 074) is unchanged; new behavior is a new function.

## Docs that move

- `docs/development.md`: the addon sets paragraph (the undo record and its folder, the edit verbs,
  `--account`, the save warning).
- `README.md`: the `set` commands, where they are listed.

## Not in this change

- The window's set switch (074); a set shared between flavors; an undo of anything but the last apply.

## Steps

1. [ ] `sets.rs`: `apply_set` keeps the replaced `AddOns.txt` files (and the names of the files it created)
       under `<home>\<flavor-id>\set-undo\`, the last apply only; `undo_apply` puts them back byte for byte
       and removes the files the apply created, refused while the client runs (023)
2. [ ] `sets.rs`: `enable`, `disable` and `drop` of one addon in a stored set, in `sets.json` only; the
       character lookup takes an account (`--account`) when one realm and name sit under two accounts
3. [ ] `set save` warns about addons of the set that are not installed and keeps them
4. [ ] CLI: `set undo`, `set enable|disable|drop <set> <addon>`, `--account` on `save` and `apply`, each
       with `--json` (`set-undo`, `set-edit`) and the voice
5. [ ] Tests in `tests/sets.rs` and `tests/cli.rs`: the apply round-trips through undo byte for byte, an edit
       changes only the named addon, the two-account case resolves with `--account`, the save warning
6. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 062 → 073

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
