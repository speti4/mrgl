---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
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

Decided 2026-10-22: the progress of a run goes to the web view through a Tauri IPC channel
(`tauri::ipc::Channel`) that each call hands to its command, one event per step (`checking`, `checked`,
`stopped`, `updating`, `downloading`, `installing`, `updated`, `failed`), rather than global events: the
events of a run belong to that call, arrive in order, and need no listener or capability of their own.

Decided 2026-10-22: the check has its own button, "Check for updates" (read-only), and is the first step of
"Update all" and of a row's "Update", which install from the check's own picks, so nothing is installed on
an old answer. The plain list stays offline: its "update available" is `update::check` run against the
GitHub source's cache alone (an `Http` that fails as an unreachable network does), so the check's version
order and states apply without a request.

Decided 2026-10-22: the cancel is read before each addon of the check and before each install, never during
one; the install in flight finishes through the engine's stage, swap and revert, so a cancel never leaves an
addon half-written. One run at a time across the window, and the marks wait while a run goes (both write
the flavor's sidecar). The per-addon lines go to the status bar as progress, a failed update is an error
(a sticky toast), the stop before the rate limit is one warning event, and the summary of the core, the one
the command line prints, is the run's finished message. The row menu pins at the installed version.

## Docs that move

- `docs/development.md`: the window's IPC surface (`update_all`, the per-row update, the mark calls, the
  progress events) and the cancel rule.
- `README.md` ("What it will do"): the window's update, if the bullet still says "task 010".

## Not in this change

- The murloc icon and the release (012); the toolbox views of the window (074); a token prompt in the
  window (the token comes from the environment, 042); the other sources (056).

## Steps

1. [x] `mrgl-app`: `update_all` and `update_one` commands over the install helper of 055 and
       `update::check` with the progress callback of 068; progress per addon as events to the web view;
       cancel between addons (an addon in flight finishes or is rolled back by the engine's journal, never
       left half-written)
2. [x] The row menu: pin, unpin, ignore, unignore, channel through the engine's five mark calls (021); the
       per-row progress slot and the row menu items of the sketch (017) enabled; `mock-data.js` and `data.js`
       keep one contract, the made-up data gains `channel`
3. [x] The rows' "update available" from `update::check`'s answer (its version order), not the cached
       compare of 009; a Forever flavor without an AddOns folder says the client has not been started yet
4. [x] The summary in the voice's two lines (kind → status bar or toast as 009 routes it); the budget stop
       line of 068 shown once
5. [x] Tests: Rust tests of the plain functions in `src/list.rs` and the command module over fixture flavors
       (several updates, a cancel, the summary), Node tests under `tests/ui/` for the menu and the progress
       slot
6. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), cut
  from the tip of task 068's branch
- 2026-10-22 — core: `update::check_until` is `update::check` with a `cancelled` callback read before each
  addon (the check ends there with the addons checked so far); `check` calls it with one that never
  cancels. `UpdateCheck::summary(SummaryOf, folder)` is the summary the command line wrote in
  `update_summary`, moved into the core so the window shares it, with `SummaryOf::Check` (`checked
  _forever_: …`) and the count a cancel left (`Update { updated, left }`, `2 not updated (cancelled)`).
  `install::Mark` with `Engine::apply_mark` and `Mark::line` are the command line's `Mark` and `mark_line`,
  moved the same way. The command line's lines are unchanged (its 115 tests pass as written).
- 2026-10-22 — `mrgl-app`: `src/actions.rs` with `UpdateRun` (one run at a time, its cancel flag, a guard
  that clears both), `run(engine, source, plan, cancelled, home, voice, events)` for `Plan::Check`,
  `Plan::All` and `Plan::One(folder)`, the events (`UpdateEvent { phase, folder, index, total, check,
  message }`), `mark_of` and `mark`. `src/commands.rs`: `check_updates`, `update_all`, `update_one` (each with
  a `Channel<UpdateEvent>`), `cancel_update` and `mark`; the stub of `update_all` is gone. `src/list.rs`:
  the rows' `update` and the new `check` come from the offline check, a row carries `channel`, a flavor
  `addons_folder`; `cached_update` is gone, and `CacheOnly` (304 to every request) became `Offline`
  (a network error to every request): the 304 had refreshed the cache's `checked_at` on every list load,
  which an offline pick then reported as the time the API last confirmed the list.
- 2026-10-22 — UI: `ui/actions.js` (the row menu's items and why one is disabled, the progress slot by
  phase, a check's answer laid over its row, the note under the version); `data.js` hands a Tauri `Channel`
  to the update commands; `index.html` gains "Check for updates" and "Cancel" next to "Update all", the
  step under the pill, and a menu the view fills; `main.js` runs one update at a time, drops an event that
  comes after its run's answer, redraws the rows of the flavor an answer carries; `mock-data.js` answers the
  same contract with a made-up check, updates (era's first one fails), cancel and marks, and its updates
  sit on GitHub rows, the source the check asks.
- 2026-10-22 — tests: `tests/update_actions.rs` (8 tests on a made-up Forever flavor of four addons
  installed from the fixture zips, against a stub source: update all with one failure, the check alone, a
  cancel during an install, a cancel during the check and before it, a row's update, the budget stop, a
  flavor without an AddOns folder, the five marks); `tests/flavor_list.rs` (the contract's new keys, the
  offline check's order, the cache left byte for byte, a flavor without an AddOns folder); the core's
  summary test; `tests/ui/actions.test.js` (8) and the data and mock tests on the grown contract.
- 2026-10-22 — the window on two made-up game folders (Verification); one finding fixed: a cancel that came
  before the first addon said `after checking 0 of 0 addons`, now `cancelled before its first addon`.

## Deviations from the plan

- The core changed besides the window: the cancellable check (`update::check_until`), and the summary and
  the mark line moved from the command line into the core (`UpdateCheck::summary`, `install::Mark`), so the
  window and the command line say the same.
- The list's cache-only `Http` answers with a network error instead of `304 Not Modified` (see the Log):
  the offline rule of task 041 reads the same cache without stamping it.
- The contract grew beyond `channel`: `Data.checkUpdates`, `Data.updateOne`, `Data.cancelUpdate` and
  `Data.mark`; `Data.updateAll(flavorId, progress)` answers `{ flavor, messages }` instead of reporting
  messages through a callback; a row carries `check`, a flavor `addons_folder`.
- The line for a missing AddOns folder holds for every flavor, not Forever alone.
- A row's "Update" is disabled, with the reason as its title, for a pinned, an ignored, a linked, a zip and a
  non-GitHub addon (the check would leave each alone).

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 176, `tests/cli.rs` 115 unchanged,
  `tests/install.rs` 33, `mrgl-app` unit 5, `tests/flavor_list.rs` 13, `tests/update_actions.rs` 8, the Node
  tests through `ui_smoke`: 59). `update_actions.rs`: update all over four addons sends checking and checked
  for each, then updating, downloading, installing and updated for `OneFolder` and `SeveralFolders` and
  failed (an error) for `RootToc`, whose zip holds no addon, and sums up `2 addons updated, 1 failed, 1 up
  to date` (a warning); `OneFolder` and `SeveralFolders` are at 1.1.0 and 2.1.0 on disk and in the sidecar,
  `SeveralFolders_Data` (no longer shipped) is gone, `RootToc` is at 3.0.0 as before, the staging folder is
  empty. A cancel at the first download lets `OneFolder` finish (1.1.0 whole on disk and in the sidecar)
  and starts no other install (`SeveralFolders` 2.0.0 with all three folders, no staging left), summary `1
  addon updated, 2 not updated (cancelled), 1 up to date`. A cancel at the first checked addon ends with
  `update cancelled after checking 1 of 4 addons in _forever_; nothing updated`, one before the first asks
  nothing. A source with two requests left sends the stop line once, as a warning, and the summary counts
  `2 not checked (rate limit)` without repeating it. The marks answer `pinned SeveralFolders at 2.0.0 in
  _forever_`, a nothing-to-do on a second pin, the unpin, ignore, unignore and channel lines of the command
  line, and the rows as they are now.
- 2026-10-22 — the window (`target/debug/mrgl-app.exe` of the worktree) on a made-up game folder and home
  (`MRGL_ROOT`, `MRGL_HOME`, `MRGL_HTTP_STUB` on the fixture answers), driven through WebView2's DevTools
  port with screenshots, then closed: the list at start showed no update (nothing cached); "Check for
  updates" put the progress lines in the status bar and ended with the warning toast `checked _forever_: 1
  update available, 1 up to date, 1 newer than the release, 1 pinned, 1 ignored, 1 source gone, 1 not
  checked`, Questie `1.2.2 → 1.2.3`, `Switched` noted `newer than the release`, `Pinned` `pinned at 2.0.0,
  2.1.0 is out`, "Update all (1)". Questie's menu offered `Update to 1.2.3`, `Pin at 1.2.2`, `Ignore`, `Follow
  prereleases`; the zip row's "Update" was disabled with `a zip in hand: …`. "Update all" hid itself for
  "Cancel", stepped Questie's slot through checking, updating, downloading, installing and updated, and
  ended `1 addon updated, …`; Questie read 1.2.3 afterwards. Pin, unpin, the channel both ways, unignore and
  ignore through the menus each answered their line as a success toast and redrew the row. A cancel sent at
  once answered `cancelling: …`, and the check ended `check cancelled after checking 0 of 0 addons` (the
  finding fixed above); a cancel with no run answered `no update is running, nothing to cancel`. A second
  made-up root whose `_forever_` has no `Interface\AddOns`: the list said `no Interface\AddOns folder in
  _forever_: the client has not been started for this flavor yet`, both buttons disabled with that title,
  and `update_all` asked anyway answered the nothing-to-do line. Seen in the dark theme. No real game folder
  was read.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` green; `python -I scripts/changelog.py --check` green after the regeneration. No
  manifest changed, no new crate.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
