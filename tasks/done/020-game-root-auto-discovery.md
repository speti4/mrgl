---
size: S
origin: wip/002-flavor-detection-and-addons-scan-with-toc-parsing.md
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M2 Window MVP
---

# 020 — Game root auto-discovery: the launcher's registry key and the common install folders

## Context

The first run of the command line and the window should find the game without a flag; task 002 settled for
`--root`, `MRGL_ROOT` and `mrgl root <folder>`. When none of those names the game folder, `home.rs` (or a
`discover` module) looks it up in the launcher's registry key (`HKLM\SOFTWARE\WOW6432Node\Blizzard
Entertainment\World of Warcraft`, `InstallPath`, through `windows-sys`'s registry functions, already a
dependency since 023) and in the usual folders (`C:\Program Files (x86)\World of Warcraft`, the same under
`Program Files`), checks that the hit is a game root (`.build.info`, at least one flavor folder), and
offers it: `mrgl list --flavor forever` works with no flag, `mrgl root` with no argument prints the hit and
says where it came from, and `mrgl root <folder>` saves it as before. The lookup sits behind a trait with a
stub so the tests stay machine-independent; the window's folder picker (009) starts from the same hit.
Done when: a machine with the launcher's key finds its root with `mrgl list --flavor forever` alone, the
lookup is stubbed in the tests, and the window's folder picker (009) can start from the same hit.

Decided 2026-10-21: discovery never writes `config.json` on its own; a discovered root is used for the run
and reported, and `mrgl root <folder>` (or the window's picker) is the one place that remembers it.

## Docs that move

- `docs/development.md` ("Fixtures" and the paragraph on `config.json` / `MRGL_ROOT`): the discovery order.
- `README.md`: the first-run line (no flag needed on a standard install).

## Not in this change

- The window's folder picker itself (009); only the core call it will use.
- A macOS lookup (`/Applications/World of Warcraft/`, M6).

## Steps

1. [x] The discovery trait (registry value, candidate folders) with the Windows implementation and the stub
2. [x] The resolution order in the CLI: `--root`, `MRGL_ROOT`, the remembered root, the discovery; the
       voice line that says where the root came from
3. [x] `mrgl root` with no argument reports the hit without saving; tests with the stub for every order
4. [x] Docs; every check green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `crates/mrgl-core/src/discover.rs`: the `RootLookup` trait (the launcher's `InstallPath`
  value, the usual folders in order), `SystemLookup` (Windows: `RegOpenKeyExW`, `RegQueryValueExW` for the
  size and the value, `RegCloseKey`; the usual folders from `ProgramFiles(x86)` and `ProgramFiles` with the
  `C:` defaults; the other targets find nothing) and the `FixedLookup` stub. `is_game_root` wants a
  `.build.info` and at least one flavor folder; a hit that is not one counts for its parent when that is
  (a launcher value may name a flavor folder). `windows-sys` gained the `Win32_System_Registry` feature,
  with its reason in `Cargo.toml`.
- 2026-10-21 — The resolution order lives in the core, once: `resolve_root` (the flag, a non-empty
  `MRGL_ROOT`, the remembered root read only when needed, then the discovery) and
  `resolve_root_of_process`, which every CLI command calls through one `game_root` function in `main.rs`
  and which the window's picker (009) starts from. `GameRoot` carries the path and its `RootOrigin`
  (`flag`, `environment`, `remembered`, `registry`, `common_folder` in `--json`); a discovered root is
  reported with the plain line `game root found in <origin>: <path>; run \`mrgl root <folder>\` to
  remember it`, as a warning-kind line of `list` and as the answer of `mrgl root` with no argument, and
  `root_origin` joins the `root` JSON document. The no-root error and the no-root answer of `mrgl root`
  name the discovery too.
- 2026-10-21 — `MRGL_DISCOVERY` replaces the lookup for one run (`off`, or `registry=<path>;<folder>;…`),
  so the end-to-end tests run the exe on the stub and never read the real registry; every CLI test sets it
  to `off` unless it tests the discovery. Core tests cover the game-root check, the registry-then-folders
  order, the parent fallback, a miss, the four-step resolution order, the variable's grammar and the
  origin names; CLI tests cover the registry hit, the flavor-folder value, the folders after a registry
  miss, a hit that is not a game root, the remembered root beating the discovery, `mrgl root` reporting a
  discovered root without writing `config.json`, and the `MRGL_ROOT` origin.
- 2026-10-21 — Docs: `docs/development.md` (the layout line of the core, the resolution order and
  `MRGL_DISCOVERY` in the fixtures section), `README.md` (no flag needed on a standard install).

## Deviations from the plan

- The lookup went into a `discover` module of its own rather than `home.rs`, with the whole resolution
  order next to it, so the window's picker and the CLI share one call; `MRGL_ROOT` moved there too.
- A launcher value that names a flavor folder counts for its parent: the key's shape differs between
  installs, and the parent fallback covers both without a second lookup.
- `MRGL_DISCOVERY` was not in the plan: the end-to-end tests run the exe, so the stub needs a switch the
  exe reads; `off` doubles as the user's way to turn the discovery off.
- A discovered root is a warning-kind line in `list` (something to look at: it is not remembered), and the
  `mrgl root` answer with no argument reports `MRGL_ROOT` as an origin as well, since it now answers "which
  folder would a run use".

## Verification

- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (core 44, CLI 30 passed, 1 ignored stand-in), `cargo +1.90 check --all-targets --locked`,
  `python -I scripts/changelog.py --check` and `cargo deny check` (the manifests changed): all green on the
  task branch.
- 2026-10-21 — One read-only look on the maintainer's machine, the debug exe: `mrgl root` with the real
  home answers the remembered root with its origin; with `MRGL_HOME` at a temporary folder it answers the
  no-root line, because this install is not registered under the launcher's key and nothing is under
  `Program Files`, so the registry and the folder paths were exercised against the real machine with a
  miss, and the hit paths against the stub in the tests. Nothing was written.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
