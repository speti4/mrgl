---
size: S
origin: session 2026-10-25
created: 2026-10-25
started: 2026-10-25
closed: 2026-10-25
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

1. [x] Baseline: `doctor`, `scan` on the four flavors, `list` with its marks, `list --json | head`, `key list`
2. [x] Sources live, read-only: GitHub and WoWInterface dry runs, the Wago and CurseForge refusals, the
       starter list's dry run
3. [x] Installs into the real `_forever_`: GitHub, WoWInterface, the starter list `--exact`, pin, update,
       ignore, rollback, remove, a zip in hand with `--as`
4. [x] Adopt: the counts of 111 on retail, classic and era; a probe on Forever through `adopt --apply`; the
       `.curseclient` refusal on era
5. [x] Era `WTF` on the command line: `backup`, `set list`, `set save`, `savedvars`
6. [x] The window over the debug port: the list, the live check and the kept answers, the marks and runs,
       the zip form (its native dialog left to a hand), era's marker warning and adopt card, the toolbox's
       sets, SavedVariables and running client, the preferences and diagnostics
7. [x] Teardown: `_forever_` empty again, the era `WTF` as before; the failures on the follow-up list; close

## Log

- 2026-10-25 — promoted, with 111 on the same branch
- 2026-10-25 — the round ran with the release build of `main` at 43224e8 (after task 127), the command line
  from a shell and the window over the debug port; the real folders were written only under `_forever_`
  (empty before the round) and, for the era `WTF`, through a backup, an apply, an undo and a cleanup whose
  files went back; the Questie bundle was removed mid-round once its TOC files turned out to hang every
  scan (the finding below), so the window's part ran over the other seven addons of the starter list
- 2026-10-25 — the follow-ups, reviewed as one list: the TOC parse finding is a task (129); the three
  observations are deleted on purpose: `import` printing its report at the end (a long import is rare, the
  command line stays usable, and `update --all` has the streaming lines), the check in flight at the rate
  limit counted as `failed` (the summary and the row say why, and the budget stop of 068 keeps the next run
  green), and the two small lines (the `Blizzard_*` name in the not-installed warning of `set save` is
  true to the set file, and a Wago page address without a key still says which key is missing).
- 2026-10-25 — the native file dialog of "Add a zip…" was not driven: the debug port does not reach a native
  dialog, and keystrokes into the foreground window are not safe on a machine in use; the form behind it
  stays covered by the browser check of task 116, and the dialog by a hand on the window

## Deviations from the plan

- The Wago paths ran keyless only (the refusal lines and no live resolve); the launch-day round (008) keeps
  the live client, and the Questie bundle is checked again once the TOC finding is fixed

## Verification

- 2026-10-25 — step 1, the baseline: `doctor` names the root as remembered, the four flavors with their build
  from `.build.info`, the CurseForge marker on retail, classic and era and not on Forever, the sidecar parsed
  on Forever and missing elsewhere, no key set; `scan` of the four folders (91, 84, 71 and 0 addons; the
  mismatch and out-of-date summaries on retail and classic); `list --flavor era` marks 45 addons `out of
  date` and 7 `interface mismatch`; `list --json | head -c 200` exits 0 and prints no panic (075);
  `key list`: `0 of 3 keys set, none shown`.
- 2026-10-25 — step 2, the sources live and read-only: `add github:… --dry-run` picks the Forever asset
  from `release.json`, names its interface and the Forever build; `add wowi:7032 --dry-run` for Forever is a
  guess (`WoWInterface lists no forever compatibility`, the five versions it lists named) and for era a pick
  on `1.15.7 (Classic)`, the page address resolves to the same id; `add wago:<slug>` and the Wago page
  address, and `add curse:<id>`, answer `source not configured` with the key's name and the `mrgl key set`
  hint; `import lists\forever-starter.json --dry-run` resolves all 8 entries live, two of them newer than
  the list names.
- 2026-10-25 — step 3, the real `_forever_` folder: `add github:…` installs (1 folder), the same `add` again
  says `installed already … nothing to do`; `add wowi:7032` downloads and installs the guess with its line,
  the TOC names 16001 so no interface warning follows; `import … --exact` installs the 8 entries at the
  listed versions (one older than the current release); `pin`, then `update --all --dry-run` and `update
  <name>` say `pinned at …, up to date` and `nothing updated: 1 pinned`; `unpin`; `ignore` → `ignored, not
  checked` in the check, `unignore`; `update --all -y` updates the one addon with a newer release (8
  folders) and says `1 addon updated, 6 up to date, 1 ignored`; `rollback` puts the previous version back
  from the kept zip and pins it, with the `mrgl unpin` hint; `remove` of the WoWInterface addon, then `add
  <its zip> --as wowi:7032` records the spec and the TOC's version (`v4.3.13-release`), and the next check
  calls it up to date against the source's `4.3.13-release`; the staging folder is empty after every run;
  `last-check.json` holds no answer while every check is `up to date` or `update available` (087).
  Failure: the Questie bundle (`Questie` and `QuestieDB`) makes every `scan` and `list` of the folder run for
  hours at one core: `QuestieDB` ships generated TOC files of 13 to 57 MB (115,989 to 408,484 `##` headers,
  each key distinct), and `Toc::set` looks every header up with a linear scan over the keys kept so far, so
  the parse is quadratic in the headers; the unpack of the zip itself takes 2 seconds. A task. Observation:
  `import` prints nothing between its first line and the end, so a long import shows no progress (a task
  candidate). Observation: the zip cache name of a version with a slash carries the 16-hex suffix (124).
- 2026-10-25 — step 4, adopt: the counts of the three CurseForge-managed folders are in 111. On Forever a
  managed addon was removed and its folder unpacked by hand from the kept zip, so it was unmanaged with
  three hints: `adopt` reports it `adopt as github:<owner>/<repo>` from the GitHub mapping (Forever build
  yes) over its WoWInterface id (build unknown) and its CurseForge id (`needs MRGL_CURSEFORGE_KEY`);
  `adopt --apply` writes the sidecar entry without a download and `list` shows it managed with that source
  and `forever:yes`. On era `adopt --apply` without `--alongside-curseforge` answers the one-manager refusal,
  exit 1, and no era sidecar appears under the mrgl home.
- 2026-10-25 — step 5, the era `WTF` on the command line: `backup` writes a dated zip (120 files) and
  `--list` shows it; `set list` says there is no set yet; `set save` from the 8 characters saves 56 addons
  (55 enabled, 1 disabled) and warns about four names no folder has; `set disable <set> <addon>` edits
  `sets.json` only; `savedvars` lists 44 files with 2 orphans. Observation: the not-installed warning of
  `set save` names a `Blizzard_*` addon of the client itself (a task candidate; 064 counts them as present
  for the dependency check).
- 2026-10-25 — step 6, the window over the debug port (Playwright over CDP, screenshots outside the repo):
  (a) the four flavors with their counts, the Forever header with build and interface and the era header
  with `managed by the CurseForge app`, the Forever column only on Forever (`yes` from the sources' caches,
  `unknown` for the WoWInterface addon), badges, `managed` pills, the `interface mismatch` and `out of date`
  marks as `list` prints them, the pinned note `pinned at 12.1.16, 12.1.17 is out` from the cache, the search
  down to one row and the no-match line; (c) the row menu's four items; pin → `pinned at …` and the menu's
  `Unpin (pinned at …)`, unpin, ignore → the `ignored` pill, unignore, `Follow prereleases` → `prereleases`,
  `Follow stable releases`, each with its toast and status line in the voice; the serious switch empties
  the murloc line and writes `serious` into `config.json`, and back; the theme cycles auto → light → dark
  → auto with the page redrawn in each (the title bar followed on the dark step); (g) "Copy diagnostics"
  puts the doctor's report on the clipboard, equal to `mrgl doctor` line for line but the summary;
  "Game folder…" opens the folder dialog, which was cancelled, and the window stayed as it was.
  (b) "Check for updates" on Forever shows "Cancel" while it runs and ends with `checked _forever_: 7 up to
  date, 1 pinned`; "Update all" with "Cancel" pressed at once ends with `update cancelled after checking 1 of
  8 addons …; nothing updated`; a second "Update all" met the anonymous GitHub budget: one row `check failed`
  with the 403 and the reset time in its note, the others `not checked (rate limit)`, the toast with the
  reset time and the `MRGL_GITHUB_TOKEN` hint, and `last-check.json` holding those answers; after a restart
  of the window every row showed its kept answer (078). Observation: the one check in flight at the limit
  is `failed`, the rest `rate-limited` (a task candidate: a 403 for the rate limit could be the same state).
  (f) the toolbox on era: the sets card lists the saved set with its counts, the target picker offers every
  character and the 8 by name, the chips show the two disabled ones apart; the apply to one character
  changed 5 lines (its file disagreed with the set on four names and the disabled one) and the undo put the
  file back (`1 AddOns.txt put back`), the `WTF` folder then identical to the backup (`diff -rq`), the undo
  button disabled afterwards; with a stand-in client running from the era folder the warning line named it
  with its pid, the apply was refused with the same line as toast and status, and the line was gone once the
  stand-in stopped; the SavedVariables card lists 44 rows, the orphans filter leaves 2, the cleanup moves
  them into a dated zip (4 files) and the summary turns to `no orphans`; the zip unpacked back left the
  `WTF` identical to the backup again.
  (c, continued, the window started with `MRGL_GITHUB_TOKEN`): "Update all" walks the row's steps
  `checking` → `downloading` → `installing` with the progress bar shown and "Cancel" in place, updates the
  one addon with a newer release (8 folders) and ends with `1 addon updated, 7 up to date`, the rows reading
  the new version; a row's "Update" on an up-to-date addon ends with `nothing updated: 1 up to date`.
  (e) on era, "Check for updates" was not run (nothing is managed there, the marker warning is the first
  line of the adopt report instead); the adopt card's "Ask the sources" fills 46 rows with their class
  pills (30 no known source, 9 without CurseForge, 5 no build for the flavor, 2 only with CurseForge), the
  marker's warning line and the summary in the status bar, the button reads `Adopt 7 addons`; pressing it
  opens the dialog `Adopt alongside the CurseForge app?` with the marker's line and the two buttons, and
  "Cancel" closes it with nothing written (no era sidecar under the mrgl home).
- 2026-10-25 — step 7, the teardown: the eight entries removed with `remove` (31 folders), the AddOns
  folder empty and the sidecar at 0 entries, `scan --json` of the folder byte-equal to the snapshot taken
  before the round; the era set removed, `set list` says none; the era `WTF` identical to the backup
  (`diff -rq`), the stand-in exe gone; the staging folder empty. What stays under the mrgl home: the zip
  cache of the installed versions, the GitHub and WoWInterface caches, the era `WTF` backup and the
  SavedVariables zip (all the user's own, under `backups\` and `cache\`).

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: 2026-10-25`
