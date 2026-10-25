---
size: S
origin: session 2026-10-25
created: 2026-10-25
started: 2026-10-25
plan: none
milestone: M1 Core + CLI
---

# 128 — A manual round on the real install after 0.1.0: the sources, the window and the toolbox

## Context

One dated round on the real game folders with the release build of `main`, under the one-at-a-time rule,
written as this task's Verification without local paths or character names: the baseline, the live sources
without a key, installs into the real `_forever_` folder (GitHub, WoWInterface, a zip in hand with its source,
pin, update, rollback, remove), adopt on Forever and the `.curseclient` refusal on era, the era `WTF` through a
backup, and the window over the WebView2 debug port (the list, the live check, the marks, the zip form, the
toolbox's three cards, the preferences). The adopt counts of 111 are taken in the same round.
Why: the 50 tasks since 0.1.0 were verified against fixtures alone; the game folders, the live APIs and the
window are where a case no fixture has shows up.
Done when: every step has a dated line in Verification, the real folders are as they were before the round,
and every failure is a task.

Decided 2026-10-25: no code change in the round, a failure is a task of its own; the window is driven over
the WebView2 debug port (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`, a
Playwright script outside the repo); no Wago key for now, so the Wago paths are the keyless ones; the era
`WTF` is written only after a backup and with the undo; the `_forever_` folder goes back to empty at the end.

## Docs that move

- none (a failure's fix is its own task, with its own docs)

## Not in this change

- The launch-day round on the live Forever client (008); the CurseForge round (096, waits for the key)
- The Wago source against the live API (needs a key the maintainer has not created yet)

## Steps

1. [ ] Baseline: `doctor`, `scan` on the four flavors, `list` with its marks, `list --json | head`, `key list`
2. [ ] Sources live, read-only: GitHub and WoWInterface dry runs, the Wago and CurseForge refusals, the
       starter list's dry run
3. [ ] Installs into the real `_forever_`: GitHub, WoWInterface, the starter list `--exact`, pin, update,
       ignore, rollback, remove, a zip in hand with `--as`
4. [ ] Adopt: the counts of 111 on retail, classic and era; a probe on Forever through `adopt --apply`; the
       `.curseclient` refusal on era
5. [ ] Era `WTF` on the command line: `backup`, `set list`, `set save`, `savedvars`
6. [ ] The window over the debug port: the list, the live check and the kept answers, the marks and runs,
       the zip form, era's marker warning and adopt card, the toolbox's sets, SavedVariables and running
       client, the preferences and diagnostics
7. [ ] Teardown: `_forever_` empty again, the era `WTF` as before; the failures on the follow-up list; close

## Log

- 2026-10-25 — promoted, with 111 on the same branch

## Deviations from the plan

- none yet

## Verification

- (pending)

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
