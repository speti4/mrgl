---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 028 — Lists as discovery: a shareable list format and a Forever starter list in the repo

## Context

The export format of 015 as a shareable list file (sources and versions, no local paths), and a
"Forever starter list" in the repo that `mrgl import` installs with one command, instead of a search
backend. Discovery without a catalog service: a list a friend sends, or the repo's own, is how a new
Forever player gets the usual addons on day one.
Done when: the starter list installs into an empty fixture flavor with `--dry-run`, and a list with a local
path is refused by the format check.

Decided 2026-10-22: the starter list lives at `lists/forever-starter.json` in the repo (GitHub entries
only, the addons the roadmap's M1 names and the ones with a public Forever release), with a README line
on how to import it; the format check is part of `import` and names the offending entry.

## Docs that move

- `README.md` ("What it will do"): the starter list and the one import command; `docs/development.md`
  ("Layout"): the `lists/` folder and the format rule.

## Not in this change

- Importing the listed version (057, on the install-path branch); entries of other sources (056); a
  search or catalog of any kind.

## Steps

1. [x] The format check in `list_file`: a local path, an absolute path or a zip entry is refused with the
       entry named; the export of 015 writes only what the format allows
2. [x] `lists/forever-starter.json` with the GitHub repositories that publish a Forever build today
       (checked against the release metadata, the check documented in the Log, no network in tests)
3. [x] Tests: the starter list into an empty fixture flavor with `--dry-run` against the stub; a list
       with a local path refused
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — `mrgl_core::list_file`: `ListEntry::not_shareable` (a source that is a zip file, a linked
  development folder (`link:`) or another local or absolute path, or a folder that is a path, as a clause
  after the entry's name); `ListFile::parse` refuses a list with such entries as
  `Error::ListNotShareable` (new, last variant), naming every one of them, before anything is resolved;
  `export` leaves an addon from a zip in hand or a linked folder out with a warning line
  (`<name> is left out of the list: its source … is a zip file`). The import's outcomes for a zip or a
  linked entry stay as they were, for a list built in code; the install path is untouched
- 2026-10-22 — `mrgl-cli`: the `export` and `import` help names the rule; no new flag or command
- 2026-10-22 — the Forever check, by hand: `gh api repos/<owner>/<repo>/releases/latest`, then the
  release's `release.json` asset (read as data, never committed) for a `forever` or `camelot` entry whose
  filename is an asset of the release, and the asset names for a `-forever` suffix; the top-level folders
  of each listed zip read from its central directory over HTTP ranges, nothing extracted. Then a live
  `mrgl import lists/forever-starter.json --flavor forever --dry-run` into an empty synthetic `_forever_`
  folder, which reads the release list the way the source does. Confirmed and in the list (repository,
  release tag, how it says Forever):
  - `Questie/Questie`, `bundle/v12.1.0+v1.0.6`: `release.json` names `forever` 16001 for
    `Questie-v12.1.0+v1.0.6.zip` (folders `Questie`, `QuestieDB`)
  - `RestedXP/RXPGuides`, `v4.11.27`: `release.json` names `forever` 16001 for `RXPGuides-v4.11.27.zip`
  - `DeadlyBossMods/DeadlyBossMods`, `12.1.16`: `release.json` names `forever` 16001 for
    `DBM-Core-12.1.16.zip`
  - `DeadlyBossMods/DBM-Vanilla`, `r833`: `release.json` names `forever` 16001 for
    `DBM-Vanilla_SoD-r833.zip`
  - `DeadlyBossMods/DBM-Dungeons`, `r268`: `release.json` names `forever` 16001 for
    `DBM-Dungeons-r268.zip` (its folders include `DBM-Party-Forever`)
  - `Tercioo/Plater-Nameplates`, `Plater-v661`: `release.json` names `forever` 16001 for `Plater-v661.zip`
  - `Nevcairiel/Bartender4`, `4.18.3.1`: `release.json` names `forever` 16001 for
    `Bartender4-4.18.3.1.zip`
  - `Nevcairiel/Mapster`, `1.14.0`: `release.json` names `forever` 16001 for `Mapster-1.14.0.zip`
- 2026-10-22 — the list's rule: GitHub entries only, a Forever build in the current release, one addon per
  job (quests, leveling guide, boss mods with their classic and dungeon modules, nameplates, action bars,
  world map), nothing that needs a dependency without a GitHub release. A Forever build confirmed, left out:
  - `michaelnpsp/Grid2`, `4.1.9`: `release.json` names `forever` 16001 for `Grid2-4.1.9-forever.zip`
    (also the asset suffix); the repository publishes a classic line next to it, and its newest release,
    `2.9.35` (`-bcc`, `-classic`, `-mists`), has no Forever zip: the live dry run failed with
    `no asset for flavor forever in github:michaelnpsp/Grid2 2.9.35`, since the source reads the newest
    release only. Out until the source looks further (a follow-up)
  - `BigWigsMods/BigWigs`, `v427.7` (`release.json` names `forever`): its classic raid modules,
    `BigWigsMods/BigWigs_Classic` `v12.1.2`, have no Forever build yet; the boss mod job is DBM's
  - `tullamods/Dominos`, `11.4.5` (`release.json`): the action bar job is Bartender4's
  - `d4kir92/MoveAny`, `v1.13.22`, and `d4kir92/ImproveAny`, `v1.0.23` (`release.json` and a `-forever`
    asset each): they move and restyle the frames the action bar addon owns
  - `funkydude/BugSack`, `v12.1.8` (`release.json`): needs `!BugGrabber`, which is not in its zip and has
    no GitHub release found
  - `kemayo/wow-silverdragon` `v2026.40`, `simulationcraft/simc-addon` `12.1.5-02`, `Total-RP/Total-RP-3`
    `3.5.2` (`release.json` each): specialist addons, not a day-one set
- 2026-10-22 — not yet (no Forever build in the current release, or no GitHub release at all):
  - Bagnon (`Jaliborc/Bagnon`): tags up to `v4.2.1`, no GitHub release
  - Baganator, Auctionator, Leatrix Plus, Leatrix Maps: no public GitHub repository with releases found
  - Details (`Tercioo/Details-Damage-Meter`): the latest GitHub release is
    `DetailsRetail.v9.1.0.8888.145` of 2021-10-11, without assets
  - `release.json` without `forever`: `WeakAuras/WeakAuras2` `5.22.1`, `BigWigsMods/BigWigs_Classic`
    `v12.1.2`, `BigWigsMods/LittleWigs` `v12.1.21`, `tullamods/OmniCC` `11.2.8`, `Nevcairiel/GatherMate2`
    `1.52.3`, `Nevcairiel/HandyNotes` `v1.6.31`, `Nevcairiel/ShadowedUnitFrames` `v4.3.9-classic`,
    `wardz/ClassicCastbars` `v1.9.1`, `Hoizame/AtlasLootClassic` `v3.2.0`, `Hekili/hekili`
    `v11.2.5-1.0.0`, `SFX-WoW/Masque` `12.1.0`, `funkydude/SexyMap` `v12.1.2`, `funkydude/BasicChatMods`
    `v12.1.0`, `enderneko/Cell` `r274-release`, `AdiAddons/AdiBags` `v1.10.29`, `jordonwow/bigdebuffs`
    `v68`, `Rottenbeer/ItemRack` `4.23`
- 2026-10-22 — the stub: `crates/mrgl-core/tests/fixtures/starter-list/`, one release per listed
  repository in the shape its current release had (tag, asset names, a `release.json` with the filename,
  `nolib` and the flavor and interface pairs only, every size 2048, no zip), apart from `releases/` so the
  fixture classification of `tests/github_source.rs` is not touched; Questie's folder keeps the encoded
  tag of its download address (`bundle/v12.1.0%2Bv1.0.6`)
- 2026-10-22 — tests: `list_file` unit tests (`an_entry_with_a_path_is_not_shareable`,
  `reading_a_list_refuses_every_entry_with_a_path_by_name`,
  `export_leaves_out_a_zip_in_hand_and_a_linked_folder`); end-to-end, appended to `tests/cli.rs`:
  `the_starter_list_and_its_stub_name_the_same_repositories`,
  `the_forever_starter_list_installs_into_an_empty_flavor_with_dry_run` (every line names the list's
  version, every pick is the zip `release.json` names for `forever`, nothing written),
  `import_refuses_a_list_with_a_local_path_and_names_the_entry` (with and without `--dry-run`); the 015
  test `export_then_import_gives_an_empty_flavor_the_same_sidecar_entries` now expects the zip in hand and
  the linked folder left out of the export with a warning each

## Deviations from the plan

- Grid2 has a Forever build but is not in the list: the live dry run showed that the source picks from the
  newest release only, and Grid2's newest release belongs to its classic line (a follow-up)
- The export leaves a zip-installed or linked addon out (with a warning) rather than marking it: a marked
  entry would still be a path, and the list has to read back
- `docs/development.md` also got a paragraph under "Fixtures" for the `starter-list/` stub

## Verification

- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` and
  `cargo test --locked` green in the worktree; the new tests above pass, and the 015 export and import
  test passes with the zip in hand and the linked folder left out
- 2026-10-22 — live, against GitHub, into an empty synthetic `_forever_` folder under a temporary
  `MRGL_HOME`: `mrgl --serious import lists/forever-starter.json --flavor forever --dry-run` answered
  `dry run: 8 addons of forever-starter.json for _forever_: 8 to install; nothing written` (exit 0), every
  pick `release.json names forever (interface 16001)`, and the folder stayed empty

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: 2026-10-22`
