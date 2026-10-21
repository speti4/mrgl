---
size: S
origin: done/004-install-engine-with-state-sidecar.md
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 037 — `list` reads the sidecar: managed (source, version) and unmanaged addons

## Context

`mrgl list --flavor <id>` loads the flavor's sidecar (004) and marks every scanned addon managed (with
its source and installed version) or unmanaged (with the TOC's source hints, as today), in the summary
line and in `--json`; a missing or corrupt sidecar is rebuilt silently, as D-003 says. The `list` JSON
also carries `root_origin` (020), as the `root` document does. 004 writes the sidecar but `list` still
shows every addon as the scan sees it; the daily overview needs the managed state before `update --all`
(006) and the window (009) build on it.
Done when: a fixture flavor with one installed addon lists it as managed with source and version, the
rest unmanaged, and the JSON carries the same fields.

Decided 2026-10-21: `scan` stays the raw folder view and does not read the sidecar; `list` is the view
with the managed state.

## Docs that move

- `README.md`: the `list` usage line and what managed and unmanaged mean.
- `docs/development.md` ("Layout"): `list` reads the sidecar through `state::classify`.

## Not in this change

- The update check (006), the Forever note (011), the dependency report (029), the interface warning
  (015): each adds its own column or field later.

## Steps

1. [x] `list` loads the sidecar through the engine's state call and classifies the scan (`state::classify`)
2. [x] The plain line per addon (managed: source and version; unmanaged: the TOC hints) and the summary
       counts through the voice
3. [x] `--json`: `managed`, `source`, `version` per addon, the counts, and `root_origin` as `root` has it
4. [x] End-to-end tests on a fixture flavor with one installed addon (through the engine against a
       fixture zip) and the rest unmanaged; a corrupt sidecar is rebuilt without a word to the user
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `crates/mrgl-cli/src/main.rs`: the `List` arm resolves the flavor folder together with its
  game root (`flavor_folder_and_root`; `flavor_folder` stays as the wrapper the other commands call), scans
  it, and reads the sidecar through `Engine::new(..).load_state()` (`list_state`). `report_scan` takes an
  optional `ListView` (the state and the game root): `scan` passes none and prints exactly what it printed
  before; `list` classifies the scan with `State::classify`, one status per addon in the scan's order.
- 2026-10-21 — One builder per addon for both commands: `addon_line(addon, status)` (folder, version,
  interface list, then `managed` and the source spec as the sidecar has it, `managed by <name>` for a
  secondary folder of a managed addon, or `unmanaged`; the TOC's source hints only for an addon mrgl does not
  manage; then the flags) and `addon_json(addon, status)` (the scan's fields, plus `managed`, `name`,
  `source` and `version` with a status). The version of a managed addon is the installed one from the
  sidecar, of an unmanaged one the TOC's `## Version` (`shown_version`). The summary gains
  `: <m> managed by mrgl, <u> unmanaged` when there are addons; the `list` JSON gains `counts` (`addons`,
  `managed`, `unmanaged`), `root` and `root_origin` (null for a flavor folder given as a path). The `List`
  path did not write `root_origin` before this change; a test now covers `flag`, `environment`, `registry`
  and null.
- 2026-10-21 — A missing sidecar and a corrupt one both read as a fresh state without a word to the user,
  and `list` writes nothing: the next install or removal rewrites the file and carries the engine's one
  warning about it. A sidecar that cannot be read at all (no mrgl home, an I/O error) is a warning line, and
  every addon reads as unmanaged.
- 2026-10-21 — Tests in `crates/mrgl-cli/tests/cli.rs`: a temporary game folder with an addon put there by
  hand (a CurseForge hint in its TOC) lists it unmanaged with the hint, before and after
  `one-folder-1.0.0.zip` is installed through the engine; after the install `OneFolder` is managed with the
  zip's path as its source and `1.0.0` as its version, in the plain lines, the summary counts and the JSON
  (and keeps `1.0.0` after its TOC is edited by hand); `several-folders-2.0.0.zip` shows `managed by
  SeveralFolders` on its secondary folders; a corrupt sidecar lists everything unmanaged with no warning
  and stays untouched; `scan --json` carries none of the new fields. Three summary assertions of the
  task-002 and task-020 `list` tests gained the counts.
- 2026-10-21 — Docs: `README.md` (the `list` usage line and what managed and unmanaged mean),
  `docs/development.md` (the layout line of the CLI: `list` reads the sidecar through `state::classify`,
  `scan` stays the raw folder view).

## Deviations from the plan

- The plain line and the JSON object are built by one function each for `scan` and `list`, with the
  sidecar status as an optional argument, rather than a second pair for `list`: a later mark (011, 015, 021,
  029) is one more part or one more field in one place, and `scan` stays the raw view because it passes no
  status.
- The JSON object carries `name` beside the three planned fields: a secondary folder of a managed addon
  needs the owner's name to be read without a second lookup.
- The `list` JSON carries `root` beside `root_origin`, as the `root` document has both.
- The summary says `managed by mrgl`, so the count reads clearly next to `managed by the CurseForge app` in
  the same line.

## Verification

- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite; the core unit tests 89, the CLI end-to-end tests 55 passed, 1 ignored stand-in),
  `cargo +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check`: all green on the
  task branch. No manifest changed, so no `cargo deny check`.
- 2026-10-21 — The debug exe on the synthetic fixture tree with `MRGL_HOME` at a temporary folder: `list
  --flavor retail` prints the fourteen addons as unmanaged with their hints and the summary `14 addons in
  _retail_ (wow 12.1.5.70448, managed by the CurseForge app): 0 managed by mrgl, 14 unmanaged, 1 folder
  without a TOC skipped`; `scan` of the same folder prints its lines and summary unchanged. No real game
  folder was read; a look at the real `_forever_` folder belongs to the maintainer's manual round.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
