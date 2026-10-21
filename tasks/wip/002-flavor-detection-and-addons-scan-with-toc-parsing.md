---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 002 — Flavor detection and the AddOns scan with TOC parsing; CLI `scan`, `list` and `root`

## Context

The first product code: every later M1 task (the GitHub source, the install engine, the update check)
starts from knowing the flavor of a game folder and what is installed in it. `flavor` reads a flavor folder's
`.flavor.info` (two lines: the `Product Flavor!STRING:0` header and the id `wow`, `wow_classic`,
`wow_classic_era` or `wowf`) and the install root's `.build.info` (one pipe-delimited file for every
product, columns found by their header names, the `Version` row of the flavor's `Product`); the interface
number is derived from the version (`major*10000 + minor*100 + patch`). `toc` parses the `## Key: value`
header lines, the multi-TOC file names (game-type suffixes `_Standard`, `_Forever`, `_Camelot`, `_Mists`,
`_Cata`, `_Wrath`, `_TBC`, `_Vanilla`, the family suffixes `_Mainline` and `_Classic`, the legacy `_BCC`,
`_WOTLKC` and hyphen forms), comma-separated `## Interface` lists, the dependency keys and the
`X-Curse-Project-ID` / `X-Wago-ID` / `X-WoWI-ID` hints. `scan` lists every folder under `Interface\AddOns`
with its TOC files and the one the flavor loads (game-type suffix, then family suffix, then the base file).
`mrgl scan <folder>`, `mrgl list --flavor <flavor>` and `mrgl root [<folder>]` print it, `--json` too.

Decided 2026-10-21: an unknown `.flavor.info` id is accepted with a warning, never an error, so a new
Blizzard product does not need a code change; `list --flavor` resolves the game root from `--root`, then
`MRGL_ROOT`, then the root remembered by `mrgl root <folder>` in `%LOCALAPPDATA%\mrgl\config.json`;
the mrgl home folder honors an `MRGL_HOME` override so tests never touch the real one.

Done when: `mrgl scan` over a real retail folder lists every addon with its TOC data, the synthetic fixtures
cover every TOC shape above, and the tests run offline.

## Docs that move

- `crates/mrgl-core/tests/fixtures/README.md`: the `flavors/` tree replaces the planned `addons/<Name>/` line.
- `docs/development.md` ("Fixtures"): the hand-written `flavors/` tree, `MRGL_HOME` and `MRGL_ROOT`.
- `docs/nerglish.md` ("What stays plain"): the data lines of a listing are plain, the summary after them is
  the message.
- `README.md`: `mrgl root <folder>` and `mrgl list --flavor forever` as usage lines.
- `CLAUDE.md` ("Data outside the repo"): `config.json` next to the sidecars under `%LOCALAPPDATA%\mrgl\`.
- `crates/mrgl-core/Cargo.toml`: serde and serde_json join with their reason; `lib.rs` lists `home`.

## Not in this change

- The state sidecar and the managed / unmanaged state of an addon (004).
- The junction bookkeeping and a junction fixture (005); the scan only reports a linked folder.
- The warning when a TOC's interface does not match the folder's build (015).
- The window (009) and the root auto-discovery from the registry or Battle.net (a follow-up).

## Steps

1. [ ] `toc.rs`: suffix and file-name classification, the header parser, unit tests; serde into `mrgl-core`
2. [ ] `flavor.rs`: `FlavorId`, `BuildVersion`, `.flavor.info` and `.build.info` parsing, the TOC chain,
       `read_flavor` and `list_flavors`; `error.rs` gains `NotAGameRoot`
3. [ ] `scan.rs` with the fixture tree under `tests/fixtures/flavors/` and `tests/scan.rs`
4. [ ] `home.rs`: the mrgl home folder, `config.json` read and write; `error.rs` gains `NoHome`
5. [ ] `mrgl-cli`: `scan`, `list --flavor [--root]`, `root`, the human lines and the one JSON document,
       end-to-end tests; the docs under *Docs that move*
6. [ ] Manual round over the real flavor folders, recorded in Verification without local paths
7. [ ] Every check of CONTRIBUTING.md green, CHANGELOG regenerated, close

## Log

- 2026-10-21 — promoted. Facts checked before the work on a real install, on warcraft.wiki.gg and in the
  BigWigs packager: `.build.info` lives at the install root, not in the flavor folder; `.flavor.info` has
  a header line; the Forever folder has no `Interface` folder before launch; `_CATA` and `Name-Wrath.toc`
  shapes occur in the wild next to the underscore files

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
