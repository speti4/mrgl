---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M3 Toolbox
---

# 015 — Addon list export and import, and a TOC-interface versus build warning

## Context

Export a flavor's managed addon list (sources and versions) as a file, import it into another flavor or
machine, and warn when an installed addon's `## Interface` does not match the folder's `.build.info`. A
fresh install or a second machine wants the same list; a mismatched interface is the first sign of a
wrong flavor pick.
Done when: an export imports into an empty fixture flavor with the same result, and the warning fires on
a fixture addon with a stale interface.

Decided 2026-10-21: the list file is JSON (`mrgl-list` schema 1: the flavor it came from, one entry per
managed addon with source and version); `export` writes it or prints it to stdout, `import` resolves and
installs every `github:` entry through the source and the engine (a zip-in-hand entry is reported as
not importable, a linked one skipped), with `--dry-run` listing what would happen; the interface warning
compares the primary TOC's `## Interface` with the flavor's build (the exact major of 002's build
reading; several interface numbers in a TOC pass when one matches) and is a warning in `list` and after
an install, never a refusal.

## Docs that move

- `README.md`: the `export` and `import` usage lines.
- `docs/development.md` ("Layout"): the list file format and the interface check.

## Not in this change

- Addon sets (014), the shareable list format for discovery (028, which may reuse this file), the
  window's import button.

## Steps

1. [x] The list file: a type with its schema, written from the sidecar's managed entries, read back
2. [x] `mrgl-cli`: `export --flavor <id> [--to <file>]` and `import <file> --flavor <id> [--dry-run]`
       (the installs through the source and the engine), the voice lines, `--json`
3. [x] The interface warning: the TOC's interface against the flavor's build, in `list` and after an
       install, with a documented line
4. [x] Tests: export from a fixture sidecar, import into an empty fixture flavor against the folder
       stub, the same sidecar after; the warning on a fixture addon with a stale interface
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `mrgl_core::list_file`: `ListFile` (`format` `mrgl-list`, `schema` 1, `flavor`,
  `exported_at`, `addons` with `name`, `source`, `version`, `folders`), `ListFile::from_state`, `parse`
  (another format or schema is `Error::ListUnreadable`), `read`, `write`; `export(&Engine)` (the sidecar
  reconciled against a scan, the sidecar not written); `import(&Engine, &dyn Source, &ListFile, dry_run)`
  with one `Imported` per entry (`installed`, `would-install`, `already-installed`, `not-importable`,
  `skipped`, `failed`), the download under `staging\downloads\` removed after each install
- 2026-10-21 — `mrgl_core::interface`: `check_addon(&Addon, Option<BuildVersion>)` and `check_scan`, an
  `InterfaceMismatch` (`folder`, `toc`, `interfaces`, `build`, `major`) with its plain line
- 2026-10-21 — `mrgl-cli`: `export` and `import` with their voice lines and `--json` documents (`export`,
  `import`); `scan` and `list` mark the line `interface mismatch`, add one warning line and the JSON field
  `interface_mismatches`; `add <file.zip>` adds the warning after the install and `interface_mismatch` to
  its JSON; `import` the same per installed entry
- 2026-10-21 — fixture `zips/stale-interface-1.0.0.zip` (`## Interface: 110207`) through `make-zips.py`;
  the rerun rewrote the other zips byte for byte (`git status` clean for them)

## Deviations from the plan

- The interface warning shows in `scan` too: `scan` and `list` share one report, and the check needs nothing
  `scan` lacks.
- The check reads the TOC the flavor loads (the primary TOC of an addon the flavor loads at all): a folder
  the flavor loads no TOC from already carries `no toc for this flavor` and gets no interface warning; a TOC
  without an `## Interface` line gives none either.
- A list entry carries the addon's `folders` and the file an `exported_at` stamp beyond source and version:
  the folders let the import leave alone an addon whose folder is there unmanaged (installed by hand or by
  another manager), so an import never overwrites a folder mrgl does not manage.
- An import installs the current release of a `github:` entry, not the listed version: the GitHub source
  resolves the current release only; the line names the listed version when the two differ.
- A source the import does not handle yet (`wago:`, `wowi:`, `curse:`) is reported as not importable, like a
  zip in hand.
- `import --dry-run` writes nothing under `Interface\AddOns` or in the sidecar; the GitHub release-list cache
  under the mrgl home is written as on any resolve.
- The test `add_takes_a_flavor_folder_path_and_needs_a_flavor` now ends with the interface warning: its
  fixture `root-toc-3.0.0.zip` names the retail interface 120105, which a Forever build does not match.

## Verification

- 2026-10-21 — `cargo test --locked`: green; `mrgl-core` 98 unit tests (new: `interface` 4, `list_file` 5)
  and the `mrgl-cli` end-to-end tests 54 passed, 1 ignored (the stand-in client). The end-to-end test
  `export_then_import_gives_an_empty_flavor_the_same_sidecar_entries` exports a fixture sidecar (two zip
  installs, one entry set to `github:Questie/Questie` 1.2.3, one `link:` entry) to a file and to stdout,
  imports it into an empty `_forever_` against a copy of the folder stub with `several-folders-2.0.0.zip`
  behind the Forever asset: the dry run writes no sidecar and no folder, the import gives the
  `SeveralFolders` entry equal to the exported one (source, version, folders, file hashes; the install time
  aside), the zip in hand and the linked entry stay behind, a second import reports `already-installed`.
  `the_interface_warning_fires_after_an_install_and_in_list` installs `stale-interface-1.0.0.zip` (110207)
  and `one-folder-1.0.0.zip` (16001) into a Forever build 1.60.1: the warning fires after the first install
  and on its `list` line and JSON, never on the second; the fixture flavors `_retail_`, `_classic_`,
  `_classic_era_` give no mismatch (`interface::tests::the_fixture_flavors_have_no_mismatch`).
- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check`: green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
