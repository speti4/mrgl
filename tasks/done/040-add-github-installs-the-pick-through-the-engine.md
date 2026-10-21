---
size: S
origin: done/003-github-release-source-with-release-json.md
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 040 — `add github:<owner>/<repo>` installs the pick through the engine

## Context

`mrgl add github:<owner>/<repo> --flavor <id>` without `--dry-run` downloads the picked asset (003) to
the flavor's staging folder and hands it to the engine's one install call (004) with the pick's source
and version as the metadata; the pick's version (the tag without a leading `v`) is what the sidecar
records, the TOC's `## Version` only a fallback. The "not implemented yet" answer goes away. The two
halves are on `main`; the first M1 command a user types is this one.
Done when: `add github:Questie/Questie --flavor forever` against the stub installs the fixture asset into
a temporary AddOns folder, the sidecar names `github:Questie/Questie` and `1.2.3`, and a second run
reports it as installed already.

Decided 2026-10-21: the download goes through the source's `Http` trait so the end-to-end test runs on
the folder stub and fixture zips, never the network; the zip lands under the flavor's staging folder and
is removed once the install call returns.

## Docs that move

- `README.md`: the usage line `mrgl add github:<owner>/<repo> --flavor forever` as an install, not a
  dry run only.
- `docs/development.md` ("Layout"): the add path from the pick to the engine.
- `crates/mrgl-cli/src/main.rs` module doc: `add github:` installs.

## Not in this change

- `update` and `update --all` (006); pin, ignore and the pre-release channel (021); the release notes
  line (030); the running-client warning on install (038).

## Steps

1. [x] The download of the pick's asset through the source's `Http` into the flavor's staging folder
2. [x] The engine's install call with the pick's source and version as the metadata; the sidecar entry
3. [x] A second run reports the addon as installed already at that version (no download, no swap)
4. [x] `mrgl-cli`: the `add github:` path without `--dry-run`, the voice lines, `--json`, end-to-end
       tests against the folder stub and the fixture zips
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `install.rs`: `Engine::install_pick(source, pick, progress)` reads the sidecar, reconciles
  it with the AddOns folder in memory and looks for an entry from the pick's source (ASCII
  case-insensitive); at the pick's version it answers `PickInstalled::AlreadyInstalled` with the entry and
  downloads nothing. Otherwise it creates a fresh `staging\download-<asset>-…` folder, reports
  `PickStep::Downloading`, fetches the asset through `Source::fetch` (the 003 call over the `Http` trait),
  reports `PickStep::Installing` and calls `Engine::install` with `Metadata { source: pick.source, version:
  pick.version, name: the sidecar's name for that source }`; a guard removes the download folder on every
  way out, the engine keeps its own copy under `cache\<addon>\<version>.zip`.
- 2026-10-21 — `mrgl-cli`: `add github:` without `--dry-run` builds the engine of the flavor (the game
  root as for `add <file.zip>`) and calls `install_from_source`, which resolves, prints the progress lines
  (`resolving …`, `pick for …`, `downloading <asset>`, `installing <asset> into <flavor folder>`) and hands
  the pick to the engine; `add_command` prints the warnings and `installed <name> <version> (…) into …`,
  `updated <name> from <old> to <new> (…) in …` or `<name> <version> is installed already in <flavor folder>
  from <source>, nothing to do`. `--json` prints one `add` document: `source`, `flavor`, `build`,
  `dry_run: false`, `pick`, `installed`, `already_installed`, `addon` (`name`, `version`,
  `previous_version`, `source`, `folders`, `removed_folders`) and `warnings`. The "not implemented yet (task
  040)" answer and its test are gone.
- 2026-10-21 — fixture: `zips/make-zips.py` also writes
  `releases/github.com/Questie/Questie/releases/download/v1.2.3/Questie-v1.2.3-forever.zip` (486 bytes, one
  `Questie` folder whose TOC says `## Version: v1.2.3`), so the folder stub serves the pick's download; a
  rerun of the script leaves every other zip byte for byte as it was.

## Deviations from the plan

- The check and the download live in the core (`Engine::install_pick` with a progress callback) rather
  than in the command line alone, so the window and the import of task 015 take the same path.
- An update keeps the name the sidecar has for the source (`Metadata.name`), so a release whose main
  folder changed name updates its entry and removes the old folders instead of adding a second addon.
- In the install path the pick line is a progress message (the dry run keeps it a success); a guessed pick
  is a warning, printed with the other warnings before the summary and listed in the JSON `warnings`.
- `add github:` without `--dry-run` needs the flavor folder (it resolves the game root like `add
  <file.zip>`); the dry run still works on the flavor id alone.

## Verification

- 2026-10-21 — `cargo test --locked`: every test green. `tests/cli.rs`
  `add_github_installs_the_pick_and_a_second_run_finds_it_installed` runs `mrgl --serious add
  github:Questie/Questie --flavor forever` against the folder stub with `MRGL_ROOT` at a temporary game
  folder and `MRGL_HOME` at a temporary home: the five lines are `resolving github:Questie/Questie for
  forever`, `pick for forever: github:Questie/Questie v1.2.3 (Questie-v1.2.3-forever.zip, flavor forever,
  interface 16001): release.json names forever (interface 16001)`, `downloading Questie-v1.2.3-forever.zip`,
  `installing Questie-v1.2.3-forever.zip into _forever_`, `installed Questie 1.2.3 (1 folder: Questie) into
  _forever_`; `Interface\AddOns\Questie\Questie.toc` is there (its TOC says `v1.2.3`), the sidecar names
  `github:Questie/Questie` and `1.2.3`, the staging folder is empty and `cache\Questie\1.2.3.zip` exists. The
  second run prints three two-line messages ending in `Questie 1.2.3 is installed already in _forever_ from
  github:Questie/Questie, nothing to do`, no `downloading` line, and the sidecar's bytes are unchanged.
  `add_github_updates_another_version_and_json_is_one_document` checks the `--json` document, then sets the
  sidecar's version to `1.2.2` and gets `updated Questie from 1.2.2 to 1.2.3 (1 folder: Questie) in
  _forever_`, then `installed: false, already_installed: true` with `previous_version: 1.2.2`.
  `add_github_without_dry_run_needs_the_game_folder_and_the_asset`: no game root is `no game root for flavor
  forever: …` (exit 1); an asset the stub does not serve is `GitHub answered 404 for …/Thing-3.0.0-forever.zip`
  (exit 1) after the `downloading` line, with no `installing` line, an empty staging folder and no sidecar.
  `tests/install.rs` drives `Engine::install_pick` directly: the two progress steps in order (the download
  under `staging\`, the zip on disk at the install step), `AlreadyInstalled` for the same pick and for the
  source spelled `github:questie/questie` with one asset request in all, a reinstall once the folder is
  deleted by hand (with the engine's `deleted by hand` warning), and an update under the sidecar's name for
  the source that removes the folder the new zip lacks.
- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
