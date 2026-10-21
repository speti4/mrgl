---
size: S
origin: done/020-game-root-auto-discovery.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 043 — A second registry source for the game root: the Uninstall entry, and expanded values

## Context

The discovery of 020 also reads the Uninstall entry's `InstallLocation`
(`HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\World of Warcraft`, and the same under `HKCU`)
after the launcher's key, for an install the launcher's key does not register; and a `REG_EXPAND_SZ` value
from either key is expanded (`ExpandEnvironmentStringsW`, the `Win32_System_Environment` feature) instead of
read as written. The stub of 020 covers both in the tests. An install moved or restored by hand carries
the Uninstall entry but not the launcher's key, and then `mrgl list` asks for a flag it should not need.
Done when: the stub's Uninstall value finds the root when the launcher's value is absent, the origin says
so, and an expanded value resolves to the same root as its literal.

Decided 2026-10-22: paired with 051 on one branch.

## Docs that move

- `docs/development.md` (the game root discovery): the order of the sources and the expansion rule.

## Not in this change

- A chosen root in the window (009); a root under a non-Windows path (036).

## Steps

1. [x] The Uninstall entry under `HKLM` and `HKCU` as the sources after the launcher's key, with its own
       origin name
2. [x] `REG_EXPAND_SZ` values expanded through the stubbed environment call
3. [x] Tests on the stub: the Uninstall value alone, the launcher's value first, an expanded value
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), paired
  with 051
- 2026-10-22 — `crates/mrgl-core/src/discover.rs`: `UNINSTALL_KEYS` (the `World of Warcraft` key under
  `Microsoft\Windows\CurrentVersion\Uninstall` in the 32-bit view of `HKLM`, the native view, then
  `HKCU`) and `UNINSTALL_VALUE` (`InstallLocation`). `RootLookup` gains `uninstall_locations()` (a default
  of none, so another lookup keeps compiling); `SystemLookup` reads each key in order, `FixedLookup` has
  an `uninstall_locations` field. `discover_root` tries them after the launcher's value and before the
  usual folders, with the same rule (a game root, or the parent of a flavor folder), and reports
  `RootOrigin::UninstallEntry`: `uninstall_entry` in JSON, `the game's Uninstall registry entry` in the
  found line, `found in the game's Uninstall registry entry` in the doctor's root line.
- 2026-10-22 — Expansion: the registry reading now keeps the value's type (`RegistryText`: the text and
  whether it is a `REG_EXPAND_SZ`, through `registry_text`; `registry_string` keeps its shape on top of
  it). `registry_path(value, expand)` expands an expandable value through the call it is given and takes a
  `REG_SZ` value as written, `%` and all; a failed or empty expansion is no value. `expand_environment` is
  `ExpandEnvironmentStringsW` (a size call, then the text, with a retry when the environment grew in
  between); `SystemLookup` passes it for both keys. `Cargo.toml`: the `Win32_System_Environment` feature
  of `windows-sys`, with its reason in the comment; no new crate, `Cargo.lock` unchanged.
- 2026-10-22 — The stub: `MRGL_DISCOVERY` takes `uninstall=<path>` for an Uninstall value and
  `registry-expand=<text>` and `uninstall-expand=<text>` for an expandable value, which go through the
  same `registry_path` and `expand_environment` as the system's values.
- 2026-10-22 — `crates/mrgl-cli/src/main.rs`: the no-root messages say `none in the registry or the usual
  install folders`, and the `--root` and `root` help names the registry with both keys.
- 2026-10-22 — Tests: in the core, the Uninstall value alone finds the root with its origin, found line
  and JSON name; a launcher's value that is no game root falls through to it, a missing value is skipped
  for the next one, and a flavor folder counts for its parent; the launcher's value wins when both are game
  roots; the Uninstall entry comes before the usual folders; an expandable value through a stubbed
  environment resolves to the root of its literal, for the launcher's key and the Uninstall entry, while a
  plain string keeps its `%` and a failed or empty expansion is none; on Windows, the real environment call
  expands a set variable, keeps an unset one, and the stub's expandable items go through it. End to end:
  `root` and `list --json` with the Uninstall value alone, the launcher's value winning and the doctor's
  root line naming the Uninstall entry, and `registry-expand=` and `uninstall-expand=` items with a
  variable set on the child process resolving to the root of the literal.
- 2026-10-22 — Docs: `docs/development.md` (the Layout line, the doctor's root line with the new origin,
  the resolution order with the Uninstall entry, and a paragraph on the entry's keys, the expansion rule
  and the stub items), `README.md` (where the game folder is found).

## Deviations from the plan

- The Uninstall entry is read in the 32-bit view of `HKLM` first (`SOFTWARE\WOW6432Node\…`): the
  installer writes it there, as it writes the launcher's key, so the native path the Context names alone
  would miss it on a 64-bit Windows; the native `HKLM` view and `HKCU` follow.
- The expansion is stubbed at the call, not inside `FixedLookup`: `registry_path` takes the environment
  call as a parameter, and the end-to-end stub's `-expand` items run the real call on a variable the test
  sets on the child process, so the tests cover the expansion without a registry and without changing
  this process's environment.
- The no-root messages and the help name the registry rather than the launcher's key, since a run now reads
  two keys; two existing end-to-end assertions follow the new wording.

## Verification

- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (core 145 passed, CLI 86 passed and 1 ignored stand-in, the other suites green), `cargo +1.90
  check --all-targets --locked`, `cargo deny check` (advisories, bans, licenses and sources ok),
  `uvx zizmor .github` (no findings) and `python -I scripts/changelog.py --check`: all green on the task
  branch.
- 2026-10-22 — One look on the maintainer's machine, the debug exe with the real registry and an empty
  temporary `MRGL_HOME` (so no remembered root): the launcher's key is not there, and `mrgl root` answered
  `game root found in the game's Uninstall registry entry: <path>`, `root_origin: uninstall_entry` in
  `--json`; the real home folder and its config were not touched.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
