---
size: S
origin: done/013-wtf-backup-and-restore.md
created: 2026-10-21
started: 2026-10-22
plan: none
closed: 2026-10-22
milestone: M1 Core + CLI
---

# 045 — Pre-launch manual round on the real install: the commands no fixture covers

## Context

One dated round on the real game folders before the Forever launch, under the one-at-a-time rule, written as
this task's Verification without local paths: `add <zip>` and `remove` (004) on the real `_forever_` folder,
once with the game on another drive than the mrgl home; `backup`, `--list` and `--restore` (013) on a
flavor with a real `WTF`; `scan` and `list --json` with the client running (023); `scan` on the era folder
without a `.build.info` row (019); `add github:… --dry-run` (003) against a real repository with a Forever
release; `mrgl doctor` (024); `link` and `unlink` (005) with a real development folder; `savedvars` (025) on
a real `WTF`, its `Blizzard_` and letter-case rules; and whether the client loads an addon whose required
dependency is only a library embedded in its own folder (029's rule). The fixtures prove the code; these
touch the real folders, registry, API and client, before launch day (008).
Done when: every command above has a dated line in Verification, and every failure is a task.

Decided 2026-10-22: run with the release build of the CLI after the merge of tasks 064 to 072, one command
group per step; a failure is written here as a line and becomes a task through the wave's follow-up list.

## Docs that move

- none (a failure's fix is its own task, with its own docs)

## Not in this change

- The launch-day round on the live Forever client (008); any code change (each failure is a task of its own);
  the window (its manual check is part of 010's merge).

## Steps

1. [x] `doctor`, `scan` on every flavor, `scan` on the era folder's build line (019), `list --json`
2. [x] `add github:… --dry-run` against a real repository with a Forever release (003)
3. [x] `add <zip>` and `remove` on the real `_forever_` folder, once with the mrgl home on another drive
4. [x] `backup`, `backup --list`, `backup --restore` on a flavor with a real `WTF` (013)
5. [x] `scan` and `list --json` with the client running (023)
6. [x] `link` and `unlink` with a real development folder (005)
7. [x] `savedvars` on a real `WTF`: the `Blizzard_` and the letter-case rules (025)
8. [x] the embedded-library dependency on the client (029's rule); every failure listed; close

## Log

- 2026-10-22 — promoted; the round runs in the main checkout on its own branch, guided step by step
- 2026-10-22 — the round ran in ten steps with the release build after the merge of 064 to 072 (#113); the
  Bartender4 release zip, a probe addon and the backup zips were the only things written, and the real
  folders are as they were before the round (the probe removed, the test install removed, the restored
  `WTF` identical to the one moved aside)
- 2026-10-22 — the two failures are tasks: 075 (the closed pipe) and 076 (the embedded dependency and the
  out-of-date interface number, together with the client's `Out of date` observation); the other
  observations are deleted on purpose: the backup line's unpacked size against the list's zip size and the
  local-time name against the UTC column (both are facts, differently phrased), and a linked addon's raw
  `@project-version@` placeholder (the TOC says so, and a developer knows the packager fills it)

## Deviations from the plan

- The era client was updated by the launcher during the round, so the exe-version fallback of 019 is covered
  by the doctor and the scans before the update, and the running-client checks ran on the new build.
- The embedded-library check used a made-up probe addon copied into the era folder, since none of the
  installed addons has an embedded-only required dependency; the Forever client cannot start before the
  launch.

## Verification

- 2026-10-22 — `doctor` (release build, after the merge of 064 to 072): the root from the remembered
  `config.json`, all four flavors with their build, the era build read from `WowClassic.exe` (019), the
  CurseForge app marker `yes` on retail, classic and era and `no` on Forever, the profile folder as `~`, the
  WebView2 version, the anonymous rate limit `60 of 60`, `MRGL_GITHUB_TOKEN not set`; no finding.
- 2026-10-22 — `scan` on `_forever_`: `no addons` (the client has not been started yet). `scan` on
  `_classic_era_`: 71 addons, version from the exe, `managed by the CurseForge app`, 7 retail-only Details
  modules in the interface warning, the flavor-specific TOC sets as `no toc for this flavor`; no finding.
- 2026-10-22 — `list --flavor era --json`: valid JSON, 71 addons, 0 managed, no missing dependency (064);
  observation, not a failure: `client_running` is `null` with no client running.
- 2026-10-22 — `add github:Nevcairiel/Bartender4 --flavor forever --dry-run` against the real repository:
  the pick 4.18.3.1 from `release.json` (`names forever (interface 16001)`), the release notes link, Forever
  build `yes`, nothing written; the release list and the per-tag file land in the cache.
- 2026-10-22 — `add <zip>` (the Bartender4 release zip in hand) into the real `_forever_`: `installed
  Bartender4 4.18.3.1 (1 folder)`, the folder under `Interface\AddOns`, the sidecar with the zip as source
  and the file hashes. `remove Bartender4`: the folder gone, the sidecar `addons: {}`.
- 2026-10-22 — the same `add` and `remove` with `MRGL_HOME` on another drive than the game: the sidecar and
  the zip cache in that home, the staging folder empty after the install, the folder on the game's drive;
  with no `config.json` in that home the root came from the game's Uninstall registry entry (043), with the
  hint to run `mrgl root`.
- 2026-10-22 — `backup --flavor era` on the real era `WTF`: 97 files into a dated zip under the mrgl home's
  `backups\wow_classic_era`; `--list` shows it with its file count, size and time; `--restore <name>`
  without `-y` and without a terminal answers the confirmation with `restore cancelled, nothing changed`;
  with `-y` it moves the current `WTF` aside as `WTF-before-restore-<date>` and unpacks the zip: the
  restored folder is identical to the one moved aside (`diff -rq`, 97 files). Observations, not failures:
  the backup line reports the unpacked size, the list the zip's size; the zip name is local time, the list
  column UTC.
- 2026-10-22 — with the era client running (the launcher updated it on start, so its build now comes from a
  `.build.info` row; the exe fallback of 019 was covered by the scans before): `scan` ends with `client
  running`; `list --json` carries `client_running` with the exe and the pid (and `null` without a client);
  `backup --restore -y` is refused with the client's name and pid, nothing under `WTF` written. Failure: `list
  --json | head -c 200` panics with `failed printing to stdout` (os error 232, the closed pipe) instead of
  exiting quietly; a task.
- 2026-10-22 — `link` of a real development folder (two addons, on another drive than the game) into
  `_forever_`: two junctions under `Interface\AddOns` with the development folder as target; `list` shows both
  as `managed`, `link:<folder>`, `linked`, Forever build `unknown`; `unlink` removes both, the AddOns folder
  is empty, the development folder untouched (its git status clean), the sidecar `{}`. Observation, not a
  failure: the version column shows the TOC's raw `@project-version@` placeholder of a packager-built addon.
- 2026-10-22 — `savedvars --flavor era` on the real `WTF`: 40 files, sizes per account and per character, the
  `Blizzard_*` files marked `client` and never orphans, the two orphans named are addons with no folder among
  the 71 installed; `--clean` not run.
- 2026-10-22 — the embedded-library dependency on the client (029's rule): a made-up addon whose only
  `## Dependencies` entry is a library under its own `Libs\` folder, copied into the era AddOns folder.
  `list` shows it without a dependency warning (the core counts the library as `embedded`); the client's
  AddOn list shows it as `Dependency missing` and does not load it. Failure: the 029 rule disagrees with the
  client, so an embedded-only required dependency is missing; a task (the reopen condition of 052). The
  probe folder was removed afterwards; nothing of it remains under `AddOns` or `WTF`. Observation: after the
  launcher's client update every addon with the previous interface number (11507 against 11509) shows as
  `Out of date` in the client, while `scan` warns only about another expansion's number; whether `scan`
  should say "out of date" for a lower number of the same expansion is a question for the follow-up list.

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: 2026-10-22`
