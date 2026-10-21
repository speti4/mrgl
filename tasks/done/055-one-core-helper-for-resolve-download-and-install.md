---
size: M
origin: done/015-list-export-import-and-interface-warning.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 055 — One install path: a core helper for resolve, download and install, the link refusal in the commit

## Context

The resolve, download and engine-install steps that `add github:` (040, `Engine::install_pick`) and
`import` (015, `list_file::import`) each wrote fold into one core function both call, with one staging
folder rule and one already-installed rule, that resolves a managed addon on its own channel (021,
`Target::with_channel`) instead of the stable default; the refusal to install over a linked addon or a
foreign junction (005, `refuse_links`) moves from `Engine::install` into `Engine::commit`, so every path
that stages and commits is covered; and the CLI's install summary line and JSON block become one helper.
The one path that writes the user's folder was written twice and is about to be written a third time
(`update --all`, 006); a caller that forgets the link check overwrites a junction into a development folder.
Done when: one core function with three callers (`add github:`, `import`, `update`), the check once in
`commit`, one summary helper, and the CLI tests of the existing paths unchanged.

Decided 2026-10-22: this task is the first of the chain 055 → 006 → 057 on one branch; `update --all`
(006) is the third caller and lands in the same pull request.

## Docs that move

- `docs/development.md` ("Layout"): the one install helper named as the path every source-backed install
  takes, and where the link refusal lives.

## Not in this change

- The update check and `update --all` themselves (006, next in the chain); `import --exact` (057).
- Any change to the staging, swap or revert rules of the engine (004), to the marks (021) or to the link
  module (005) beyond moving the one refusal.

## Steps

1. [x] The core helper (resolve on the entry's channel, download, stage, commit) with `add github:` and
       `import` as its callers; the duplicated code in `Engine::install_pick` and `list_file::import` removed
2. [x] `refuse_links` called once in `Engine::commit`; the tests of 005 (install over a linked addon, over a
       foreign junction) pass through every path
3. [x] One CLI summary helper (the plain lines and the JSON block) for every install path
4. [x] The existing CLI tests of `add github:` and `import` unchanged and green
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 055 → 006 → 057
- 2026-10-22 — `install.rs`: `Engine::install_from_source(source, &SourceRequest, progress) ->
  SourceInstalled { pick, outcome }` is the one path. `SourceRequest { spec, wanted, name }` names the
  source, which release (`Wanted::Current`, or `Wanted::Pick` for a pick made already) and the name the
  sidecar records when it has no entry from that source. The request is resolved on
  `Engine::target_for(state, source, name)`: the flavor's build on the channel of the sidecar entry from
  that source (ASCII case-insensitive), else of the entry of that name, else `stable`. The progress
  callback gains `PickStep::Resolving` and `PickStep::Picked` before `Downloading` and `Installing`. The
  already-installed rule (an entry from the pick's source at the pick's version, folders there) and the
  staging rule (a fresh `staging\download-<asset>-…` folder removed on every way out) are written once;
  the download is staged and committed directly, so the AddOns folder is scanned once per install instead
  of twice. `Engine::resolve(source, &SourceRequest)` is the resolve alone (dry runs);
  `Engine::install_pick(source, pick, progress)` stays as the same path for a pick in hand
  (`Wanted::Pick`).
- 2026-10-22 — `Engine::commit` calls `refuse_links` before the first folder moves; `Engine::install` no
  longer does. `Engine::rollback` staged and committed without the guard before; it refuses now too. With
  the guard taken out of `commit` for a trial run, the new test installed over the linked `OneFolder` (the
  junction went into the staging folder), which is the case the move closes.
- 2026-10-22 — `list_file::import` calls `Engine::resolve` on `--dry-run` and `Engine::install_from_source`
  otherwise, with the list's name in the request; its own `staging\downloads` folder and fetch-then-install
  code are gone. The import's skip rule (by name, by source, or a folder of the entry that is there) stays
  the import's: it decides which entries reach the install path at all.
- 2026-10-22 — `mrgl-cli`: `install_progress` is the progress of the one path (the resolve, the pick or the
  guess warning, the download, the install) and `install_summary` / `already_installed_summary` give every
  install path its summary line and its `addon` JSON block with the dependency and interface checks; `add
  github:` and `add <file.zip>` use them, and the command-line `install_from_source` is gone.
- 2026-10-22 — tests: `tests/link.rs` `every_install_path_refuses_a_linked_addon_and_a_foreign_junction`
  (a source without the network serving a fixture zip); `tests/install.rs`
  `the_one_path_resolves_a_managed_addon_on_its_channel_and_finds_it_installed`; the core test of the
  import checks that the staging folder is empty instead of the old `downloads` folder.

## Deviations from the plan

- `Engine::install_pick` stays as a thin entry to the one path for a pick in hand (`Wanted::Pick`), so the
  window and the update check can install a pick they made already without a second request; its
  progress callback has the two new steps, which a pick in hand never reports.
- The already-installed rule of the install path is one; the import keeps its own wider skip rule in front
  of it (an addon the flavor has by name, by source or by a folder is left alone at any version), as task 015
  decided.
- `add <file.zip> --dry-run` still calls `refuse_links` itself, as a preview of the refusal the commit makes,
  since a dry run never commits.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 141, `tests/install.rs` 29, `tests/link.rs`
  7, `tests/cli.rs` 82 with 1 ignored). The CLI tests of `add github:` and `import` are unchanged and green:
  `add_github_installs_the_pick_and_a_second_run_finds_it_installed` still sees the five progress and
  summary lines in order and the four two-line messages of the second run;
  `add_github_updates_another_version_and_json_is_one_document`,
  `add_github_without_dry_run_needs_the_game_folder_and_the_asset`,
  `export_then_import_gives_an_empty_flavor_the_same_sidecar_entries` and
  `import_failures_and_unreadable_lists_are_exit_code_one` pass as written. The new link test refuses
  `OneFolder` linked from the development folder through `install_from_source` (`refused to install over
  OneFolder (flavor forever): it is linked from …`) and through the import (outcome `failed` with that
  line), then, with a junction made by hand over a managed `OneFolder` 1.1.0 that has a previous version,
  through `install` (zip), `install_from_source`, the import and `rollback` (`Error::ForeignJunction`); the
  junction is still there, the entry is still 1.1.0 and unpinned, no `Fresh` entry was written, and the
  development folder is byte for byte as it was. The channel test resolves `github:Example/Prerelease` to
  1.0.0 for an addon the sidecar lacks and to 1.1.0-beta for an entry on the prerelease channel (found by its
  source spelled `github:example/prerelease`), reads the channel by name for an entry of another source,
  and stops with `AlreadyInstalled` after the `Resolving` and `Picked` steps when the sidecar has
  1.1.0-beta, with an empty staging folder.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` and `cargo +1.90
  check --all-targets --locked` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
