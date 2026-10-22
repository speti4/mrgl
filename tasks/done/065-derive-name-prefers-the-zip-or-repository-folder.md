---
size: S
origin: done/028-lists-as-discovery-and-forever-starter-list.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 065 — The install engine names a multi-folder addon after the zip or the repository, not the first folder

## Context

When no folder of a zip is extended by all the others, `install::derive_name` prefers the folder named
after the zip's stem or the source repository before falling back to the first folder by name. The DBM-Core
zip is recorded as `DBM-Brawlers`, so `mrgl add github:DeadlyBossMods/DeadlyBossMods` installs under a name
nobody looks for.
Done when: a fixture zip shaped like DBM-Core is recorded under its core folder's name, and the existing
naming tests are unchanged.

Decided 2026-10-22: second of the chain 064 → 065 → 066 → 067 on one branch.

## Docs that move

- `docs/development.md`: the naming rule of a multi-folder addon, where it is stated.

## Not in this change

- Renaming an addon already recorded in a sidecar under the old name; the `## Title` of a TOC as a name
  source.

## Steps

1. [x] `install.rs`: `derive_name` takes the zip's stem and the repository name as hints and prefers a folder
       that matches one of them (case-insensitive, with the flavor suffix and version stripped from the stem)
       before the alphabetical fallback; `Engine::stage` passes the hints
2. [x] Tests: a fixture zip shaped like DBM-Core (`DBM-Brawlers`, `DBM-Core`, `DBM-GUI`, …) is recorded as
       `DBM-Core`; the existing naming tests unchanged
3. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; second of the chain 064 → 065 → 066 → 067
- 2026-10-22 — `install.rs`: `derive_name(folders, hints)` keeps the folder every other one extends as its
  first rule, then takes the first hint that names a folder (ASCII case-insensitive), then the first folder
  by name. `name_hints(zip_stem, source)` gives the hints, the strongest first: `stem_name(zip_stem)`, the
  `-`-separated parts before the first one that starts with a digit or with `v` and a digit (`DBM-Core` of
  `DBM-Core-12.0.11-classic`, `Questie` of `Questie-v1.2.3-forever`), or, with no version part, the stem
  without its trailing flavor suffixes and `nolib`; then the repository of a `github:` source.
  `Engine::stage` passes the hints from the zip's own stem (a source download is saved under the asset's
  name) and `Metadata::source`, so a zip in hand and a source install both take them. The flavor suffixes
  are the GitHub source's list (`sources::github::FLAVOR_SUFFIXES`, now `pub(crate)`).
- 2026-10-22 — fixture: `boss-core-5.0.0.zip` (`make-zips.py`), the shape of a boss-mod core zip under
  made-up names: `Boss-Arena`, `Boss-Core`, `Boss-GUI`, none extended by all the others and the first by
  name not the core.
- 2026-10-22 — tests: `install::tests::a_hint_names_the_addon_when_no_folder_is_extended_by_the_others` (the
  DBM-shaped folder set: `DBM-Brawlers` without hints, `DBM-Core` from the stem, from a lower-case stem with
  a flavor suffix and from the repository; the stem before the repository; no hint that names a folder
  falls back; the extension rule before any hint) and `the_stem_name_drops_the_version_and_the_flavor`;
  `tests/install.rs` `a_zip_without_a_folder_the_others_extend_is_named_after_its_stem_or_repository` (the
  fixture zip as `Boss-Core`, and a copy named `download.zip` from `github:Someone/boss-core` as
  `Boss-Core`). `the_name_is_the_folder_the_others_extend_else_the_first` keeps every expectation and only
  passes the empty hint list; its DBM case never held `DBM-Brawlers`, so it did not encode the bug.
- 2026-10-22 — docs: `docs/development.md` states the naming rule after the one install path and lists the
  new fixture zip.

## Deviations from the plan

- The hints come from the zip's own stem inside `Engine::stage` rather than as new parameters of `stage`:
  a source download is saved under its asset's name, so the stem is the asset name either way, and every
  caller of `stage` (a zip in hand, a source, a rollback with its recorded name) is covered without a
  signature change.

## Verification

- 2026-10-22 — `mrgl --serious add crates/mrgl-core/tests/fixtures/zips/boss-core-5.0.0.zip --flavor
  <temporary _forever_>` prints `installed Boss-Core 5.0.0 (3 folders: Boss-Core, Boss-Arena, Boss-GUI)
  into _forever_`, and `list` shows `Boss-Core  5.0.0  16001  managed` with `Boss-Arena` and `Boss-GUI`
  `managed by Boss-Core`; without hints the rule takes the first by name, as the unit test's no-hint case
  asserts for the DBM set (`DBM-Brawlers`).
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green, `tests/install.rs` 34), `cargo +1.90 check --all-targets --locked` and
  `python -I scripts/changelog.py --check` green before the close; the changelog regenerated with the close.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
