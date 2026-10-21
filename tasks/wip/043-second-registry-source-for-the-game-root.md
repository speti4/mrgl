---
size: S
origin: done/020-game-root-auto-discovery.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 043 — A second registry source for the game root: the Uninstall entry, and expanded values

## Context

The discovery of 020 also reads the Uninstall entry's `InstallLocation`
(`HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\World of Warcraft`, and the same under `HKCU`)
after the launcher's key, for an install the launcher's key does not register; and a `REG_EXPAND_SZ` value
from either key is expanded (`ExpandEnvironmentStringsW`, the `Win32_System_Environment` feature) instead of
read as written. The stub of 020 covers both in the tests. An install moved or restored by hand carries
the Uninstall entry but not the launcher's key, and then `mrgl list` asks for a flag it should not need.
Done when: the stub's Uninstall value finds the root when the launcher's value is absent, the origin says
so, and an expanded value resolves to the same root as its literal.

Decided 2026-10-22: paired with 051 on one branch.

## Docs that move

- `docs/development.md` (the game root discovery): the order of the sources and the expansion rule.

## Not in this change

- A chosen root in the window (009); a root under a non-Windows path (036).

## Steps

1. [ ] The Uninstall entry under `HKLM` and `HKCU` as the sources after the launcher's key, with its own
       origin name
2. [ ] `REG_EXPAND_SZ` values expanded through the stubbed environment call
3. [ ] Tests on the stub: the Uninstall value alone, the launcher's value first, an expanded value
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), paired
  with 051

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, at close)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
