---
size: S
origin: done/005-dev-addon-linking-with-junctions.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 062 — `link` takes a single addon folder as well as a development folder

## Context

`mrgl link <folder>` given a folder that is itself an addon (a TOC named after it) links that one
folder instead of answering "no addon in the development folder"; `unlink` matches. A developer with one
addon keeps its repository as the addon folder, not under a parent.
Done when: a fixture addon folder links and unlinks on its own, and a development folder still links
its subfolders.

Decided 2026-10-22: first of the chain 062 → 073 on one branch, in wave 6.

## Docs that move

- `docs/development.md` and `README.md`: the `link` paragraph names the single addon folder shape.

## Not in this change

- A folder that is both an addon and a parent of addons (the addon shape wins, the subfolders are its own);
  the window's link action; the addon sets (073).

## Steps

1. [x] `link.rs`: a folder with a TOC named after it is one addon to link; the sidecar entry records the
       source `link:<folder>` as for a development folder; `unlink` removes exactly that junction
2. [x] CLI `link` and `unlink` report the one addon; the messages through the voice
3. [x] Tests in `tests/link.rs` and `tests/cli.rs`: a fixture addon folder links and unlinks on its own, a
       development folder still links its subfolders
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 062 → 073
- 2026-10-22 — `scan.rs`: `read_addon_folder` reads one folder as the scan reads every subfolder (a TOC
  named after it, base or with a multi-TOC suffix); `None` for a folder without one, a drive root or a
  folder that cannot be read.
- 2026-10-22 — `link.rs`: `link` asks `read_addon_folder` of the given folder first; an addon folder is
  linked as that one addon, the junction pointing to the folder itself, under its own name, with the source
  `link:<folder>`; otherwise the subfolders are linked as before. `Shape` (`development-folder`,
  `addon-folder`) is on `Linked`, and `shape_of` answers it ahead of a link. `unlink` takes a junction
  whose target is the recorded folder itself as its own, as one that points under it, so the single
  addon's junction goes, also when the folder is gone (a dangling junction). `Error::NoAddonInDevFolder`
  reads "no TOC named after it and no subfolder with one".
- 2026-10-22 — CLI: `link` says "linking the addon folder <folder> into <flavor folder>" and ends with
  "linked the addon <name> from <folder> into <flavor folder>", or "<name> is linked from <folder> into
  <flavor folder> already" (nothing to do); `link --json` carries `shape`. `unlink` keeps its summary,
  which names the one addon ("unlinked 1 addon of <folder> from <flavor folder>: <name>"). Every line goes
  through the reporter's voice.
- 2026-10-22 — fixture `crates/mrgl-core/tests/fixtures/single-addon/SoloAddon/`: `SoloAddon_Forever.toc`
  only (the suffix shape), `SoloAddon.lua`, and an embedded `LibSolo/` with a TOC of its own, so the addon
  shape winning over its subfolders is tested.
- 2026-10-22 — tests: `tests/link.rs` `a_single_addon_folder_links_and_unlinks_on_its_own` (the one
  junction to the folder, the scan choosing `SoloAddon_Forever.toc` through it, the sidecar entry, a
  second link already linked, the development folder still linking `DevAddon` and `OneFolder` next to it,
  `unlink` of the addon folder taking only `SoloAddon`, the folder byte for byte as before, and a dangling
  junction after the folder is deleted still removed); `tests/cli.rs`
  `link_and_unlink_take_a_single_addon_folder` (the plain lines, `shape` in `--json` for both shapes,
  `unlink` naming `SoloAddon`); the error message test of `tests/link.rs` follows the new text.
- 2026-10-22 — docs: `docs/development.md` (the link paragraph of the layout, the CLI line with `shape`,
  the fixture), `README.md` (the `link` example for one addon folder and the `link` paragraph).

## Deviations from the plan

- `link --json` gained a `shape` field and the core a `shape_of` function, so the progress line can name
  the shape before the link runs; neither was in the Steps.
- `unlink`'s summary is unchanged: it already names the one addon it removed.

## Verification

- 2026-10-22 — `cargo test --locked -p mrgl-core --test link`: 10 passed, among them
  `a_single_addon_folder_links_and_unlinks_on_its_own`; `cargo test --locked -p mrgl-cli --test cli link`:
  4 passed, among them `link_and_unlink_take_a_single_addon_folder`; the existing development folder tests
  unchanged and green.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green), `cargo +1.90 check --all-targets --locked` and `python -I
  scripts/changelog.py --check` green before the close; the changelog regenerated with the close.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
