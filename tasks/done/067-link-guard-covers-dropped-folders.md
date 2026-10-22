---
size: S
origin: done/055-one-core-helper-for-resolve-download-and-install.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 067 — The link guard also covers the folders a new version drops

## Context

`Engine::commit` checks every folder of the previous version that the new one no longer has before it
moves it aside, so a dropped folder that has become a junction is refused like a staged one. Today only the
staged folders are checked; a stale folder turned junction into a development folder is moved aside and
deleted with the staging folder.
Done when: a fixture update whose old version has a folder now linked by `mrgl link` is refused and leaves
the development folder untouched.

Decided 2026-10-22: last of the chain 064 → 065 → 066 → 067 on one branch.

## Docs that move

- `docs/development.md`: the link guard's rule, if the engine's commit is described there.

## Not in this change

- The `remove` command's own guard (unchanged); a junction that appears between the check and the swap.

## Steps

1. [x] `install.rs`: `Engine::commit` computes the stale folders before the guard and passes staged and stale
       folders together to `refuse_links`
2. [x] Tests: a fixture update whose old version has a folder linked by `mrgl link` is refused, the
       development folder and the AddOns folder untouched; the existing swap and link tests unchanged
3. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; last of the chain 064 → 065 → 066 → 067
- 2026-10-22 — `install.rs`: `Engine::commit` reads the previous entry and the stale folders (the previous
  version's folders the new one lacks) first and hands the staged and the stale folders together to
  `refuse_links`, before the AddOns folder is created and before the running-client check; the swap and
  the rest of the commit are unchanged.
- 2026-10-22 — `link.rs`: `refuse_links` looks for a link entry among every entry that lists the folder,
  not only the first one `entry_for_folder` finds: a dropped folder is listed by the managed addon and, once
  linked, by its link entry too, and the managed addon sorts first, so the old lookup would have called a
  junction `mrgl link` made a foreign one.
- 2026-10-22 — tests: `tests/link.rs` `an_update_that_drops_a_linked_folder_is_refused` (Windows): version
  2.0.0 of `several-folders` installed, `SeveralFolders_Data`, which 2.1.0 drops, replaced first by a
  junction made by hand into the development folder (`ForeignJunction`), then by the junction `mrgl link`
  makes with a sidecar that also lists it under `SeveralFolders` (`LinkedAddon`, `install over`); each
  time the update to 2.1.0 is refused, the junction stays, the development folder is byte for byte as it
  was, the AddOns listing and the sidecar's 2.0.0 are unchanged. Without the change in `install.rs` and
  `link.rs` the same test fails: the update installs 2.1.0 with `removed_folders: ["SeveralFolders_Data"]`.
- 2026-10-22 — docs: `docs/development.md` (the commit's guard in the layout) and the module doc of
  `install`.

## Deviations from the plan

- The linked case needs a sidecar written by the test: `mrgl link` skips a folder another entry lists, so
  a dropped folder that `mrgl link` made arises only from a sidecar that lists it under the managed addon as
  well (written by hand, or before that rule). A junction made by hand or by another link script is the
  case a real install meets, and the test covers both.
- `refuse_links` changed too (the lookup over every entry that lists a folder), so the linked case reports
  the linked addon rather than a foreign junction.

## Verification

- 2026-10-22 — `cargo test --locked -p mrgl-core --test link`: 9 passed, among them
  `an_update_that_drops_a_linked_folder_is_refused`; with `install.rs` and `link.rs` reverted the same test
  fails with `refused: Installed { name: "SeveralFolders", version: "2.1.0", …, removed_folders:
  ["SeveralFolders_Data"] }`, the junction moved aside by the update.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green), `cargo +1.90 check --all-targets --locked` and `python -I
  scripts/changelog.py --check` green before the close; the changelog regenerated with the close.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
