---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
closed: 2026-10-21
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
- `docs/development.md` ("Layout", "Fixtures"): the hand-written `flavors/` tree, `MRGL_HOME` and `MRGL_ROOT`.
- `docs/nerglish.md` ("What stays plain"): the data lines of a listing are plain, the summary after them is
  the message.
- `README.md`: `mrgl root <folder>` and `mrgl list --flavor forever` as usage lines.
- `CLAUDE.md` ("Data outside the repo"): `config.json` next to the sidecars under `%LOCALAPPDATA%\mrgl\`.
- `crates/mrgl-core/Cargo.toml`: serde and serde_json join with their reason; `lib.rs` lists `home`.

## Not in this change

- The state sidecar and the managed / unmanaged state of an addon (004).
- The junction bookkeeping and a junction fixture (005); the scan only reports a linked folder.
- The warning when a TOC's interface does not match the folder's build (015).
- The window (009) and the root auto-discovery from the registry (020).
- A build version from the client exe when `.build.info` has no row for the flavor (019).

## Steps

1. [x] `toc.rs`: suffix and file-name classification, the header parser, unit tests; serde into `mrgl-core`
2. [x] `flavor.rs`: `FlavorId`, `BuildVersion`, `.flavor.info` and `.build.info` parsing, the TOC chain,
       `read_flavor` and `list_flavors`; `error.rs` gains `NotAGameRoot`
3. [x] `scan.rs` with the fixture tree under `tests/fixtures/flavors/` and `tests/scan.rs`
4. [x] `home.rs`: the mrgl home folder, `config.json` read and write; `error.rs` gains `NoHome`
5. [x] `mrgl-cli`: `scan`, `list --flavor [--root]`, `root`, the human lines and the one JSON document,
       end-to-end tests; the docs under *Docs that move*
6. [x] Manual round over the real flavor folders, recorded in Verification without local paths
7. [x] Every check of CONTRIBUTING.md green, CHANGELOG regenerated, close

## Log

- 2026-10-21 — promoted. Facts checked before the work on a real install, on warcraft.wiki.gg and in the
  BigWigs packager: `.build.info` lives at the install root, not in the flavor folder; `.flavor.info` has
  a header line; the Forever folder has no `Interface` folder before launch; `_CATA` and `Name-Wrath.toc`
  shapes occur in the wild next to the underscore files
- 2026-10-21 — `toc` and `flavor` with 20 unit tests; `scan` and `home` with 10 more and the integration
  test over the fixture tree (14 addons and one skipped folder in the retail fixture); the CLI with 23
  end-to-end tests, `MRGL_HOME` pointed at a temporary folder in every run
- 2026-10-21 — the manual round moved the `.curseclient` marker: the CurseForge app writes it into the
  flavor folder, not into the addon folders, so it is a property of the flavor (`Flavor.curse_client`, the
  summary line's "managed by the CurseForge app") and the fixture marker moved with it
- 2026-10-21 — the manual round found the Classic Era row missing from a real `.build.info` while the
  client exe carries the version: filed as 019; the root auto-discovery the plan left out is 020

## Deviations from the plan

- `curse_client` moved from the addon to the flavor (see the Log): the marker lives in the flavor folder.

## Verification

- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (30 core unit tests, 10 scan integration tests, 23 CLI tests, the app's tests), `cargo +1.90
  check --all-targets --locked`, `cargo deny check` (advisories, bans, licenses, sources ok), all green
  locally.
- 2026-10-21 — manual round on the real install, read-only except the remembered root: `mrgl scan` over
  the retail folder listed 91 addons with version and interface and skipped one folder without a TOC;
  every addon had a chosen TOC for the flavor; six addons carried legacy hyphen files next to their
  underscore ones; the classic folder listed 84 addons with the `_Mists` pick, the era folder 71 with the
  build unknown (no `.build.info` row, task 019), the Forever folder answered "no addons" with the
  `1.60.1` build and no error; `--json` parsed as one document. `mrgl root <game folder>` saved the root
  and named the four flavors, then `mrgl list --flavor retail` worked with no flag and `mrgl root` printed
  the saved folder. Titles with color escapes rendered plain in the JSON `title_plain`.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: YYYY-MM-DD`
