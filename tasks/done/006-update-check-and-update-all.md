---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 006 — Update check: installed state against the latest release per flavor; `update --all` with dry-run and `--yes`

## Context

Compare every managed addon's sidecar entry with its source's latest release for the flavor, report
up to date / update available / source gone / guess only, and `mrgl update --all [--dry-run] [--yes]` apply
the updates with one confirmation, each through the one install helper of 055 with the source's
download (003) and the pick's version as the metadata. The voice's progress and summary lines (`2 addons
updated, 12 up to date`); an ignored addon is skipped, a pinned one reported and left alone, each addon
resolved on its own channel (021), and the summary counts pinned and ignored addons separately. The daily
command of the first weeks after launch.
Done when: a fixture sidecar against fixture releases reports every state, `--dry-run` writes nothing, and
the summary line counts are right.

Decided 2026-10-22: second in the chain 055 → 006 → 057; `update <name>` (one addon) takes the same path
as `--all` over one entry, and the stub of `update` that named this task goes.

## Docs that move

- `README.md` ("What it will do") and `docs/development.md`: the `update` command, its flags and the states
  it reports.

## Not in this change

- The window's update button and progress (010); the offline resolve from the cache (041); the GitHub
  pagination and rate-limit edges (042); a per-addon rollback after a bad update (022 exists).

## Steps

1. [x] The update check in the core: every managed entry against its source's latest pick on the entry's
       channel, one state per addon (up to date, update available, source gone, guess only, pinned, ignored)
2. [x] `mrgl update --all [--dry-run] [--yes]` and `mrgl update <name>` through the install helper of 055;
       progress and summary lines through the voice, `--json`
3. [x] Tests: a fixture sidecar against fixture releases reports every state; `--dry-run` writes nothing;
       the summary counts; a pinned and an ignored entry
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 055 → 006 → 057
- 2026-10-22 — `mrgl-core::update` (new module): `check(engine, source, only) -> UpdateCheck { addons,
  warnings }` reads the sidecar, leaves out an addon whose folders were deleted by hand (a warning), and
  checks every entry, or the one a name or a folder names (`State::find`; an unknown name is `addon not
  found`, an unmanaged folder `… only an addon mrgl installed can be updated`). Each `AddonUpdate` has the
  name, source, installed version, channel, pin, `state`, `latest`, `detail` and the `pick`. An ignored entry
  is skipped without a request; a zip in hand, a `link:` entry and an unhandled source are `not-checked`;
  a `github:` entry is resolved on `Target::from_flavor(..).with_channel(entry.channel)`: `pinned` when
  pinned (with the current release, or the error as the detail), `up-to-date` at the same version,
  `guess-only` when the pick of another version is a guess, `update-available` otherwise; an error is
  `source-gone` for no release, no asset for the flavor or a 404/410, `failed` for anything else.
  `AddonUpdate::request()` is the `SourceRequest` of an available update with the check's pick
  (`Wanted::Pick`), `AddonUpdate::line()` the plain data line, `UpdateState::label()` the summary word.
- 2026-10-22 — `mrgl-cli`: `update --all | <name> --flavor <flavor> [--dry-run] [--yes]` (clap: exactly one
  of a name and `--all`); a progress line, a data line per addon, one confirmation for the updates (`--yes`;
  without a terminal, `std::io::IsTerminal`, or with `--json` it stops with exit 1 and says what `--yes`
  does), each update through `Engine::install_from_source` with the progress and the summary helpers of 055,
  then the summary counting every state. `--json` is one `update` document with `addons` (each with
  `updated`, the installed `addon` block and `error`), `updated`, `counts` and `warnings`. The stub answer
  and `Command::not_yet` are gone with the two tests that asserted it.
- 2026-10-22 — tests: `tests/cli.rs` gains five end-to-end tests over a fixture sidecar of ten entries, one per
  state, against the folder stub; `tests/link.rs` `an_update_over_a_foreign_junction_is_refused`;
  `update::tests` for the data lines and the request.
- 2026-10-22 — on top of task 041: `update` prints `Pick::stale_warning()` for every pick of the check that
  came from the cached release list offline, and the progress of the one install path pushes it for `add`.

## Deviations from the plan

- Two states beyond the listed six: `not-checked` (a zip in hand, a linked folder, a source not handled
  yet: no source to ask) and `failed` (the source could not be asked: offline, a rate limit, a server
  error), so an offline run never reports a source as gone.
- `guess-only` is reported and not installed by `update`, for one addon as for all: an asset picked by its
  name alone is never installed without the user asking for it, and the line names `mrgl add <source>` as
  the way to take it anyway.
- Another version counts as an update whatever its number: the source's current release on the addon's
  channel is the one to have, so a deleted release or a channel switch moves the addon to the current one.
- `update` with updates to install and no `--yes` needs a terminal for its confirmation: without one, or with
  `--json`, it stops with exit 1 and says what `--yes` does, before anything is written.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 143, `tests/link.rs` 8, `tests/cli.rs` 86
  with 1 ignored). The fixture sidecar of `update_game` holds ten entries against the folder stub over
  `tests/fixtures/releases/`. `update --all --dry-run` prints one line per state: `Beta  1.1.0-beta  up to
  date` (prerelease channel, where stable would pick 1.0.0), `Dev  dev  not checked: a linked development
  folder`, `Empty  1.0.0  source gone: no release in github:Example/Empty`, `Gone  1.0.0  source gone: GitHub
  answered 404 for …`, `Guess  2.0.0 -> 3.0.0  guess only: no release.json names forever; …`, `Ignored  1.0.0
  ignored, not checked`, `Pinned  2.0.0  pinned at 2.0.0, 2.1.0 is out, left alone`, `Plain  1.0.0  up to
  date`, `Questie  1.2.2 -> 1.2.3  update available`, `Zipped  1.0.0  not checked: a zip in hand; …`, then
  `dry run: 1 update available, 2 up to date, 1 pinned, 1 ignored, 1 guess only, 2 source gone, 2 not
  checked; nothing written`; every file under the flavor folder and the sidecar's bytes are unchanged and
  there is no staging folder. `update --all --yes` prints `updating Questie from 1.2.2 to 1.2.3`, the
  download and install lines, `updated Questie from 1.2.2 to 1.2.3 (1 folder: Questie) in _forever_` and `1
  addon updated, 2 up to date, 1 pinned, 1 ignored, 1 guess only, 2 source gone, 2 not checked`; the
  Questie folder holds the release's TOC, the sidecar says 1.2.3 with previous 1.2.2, the pinned, ignored,
  guess and gone entries are unchanged, the staging folder is empty; a second run ends `nothing updated: 3
  up to date, …`. Without `--yes` (stdin is no terminal in the test) the command exits 1 with `1 addon to
  update, nothing updated: no terminal to answer the confirmation; --yes updates without asking`, and with
  `--json` with the JSON error naming `--yes`; nothing is written. The `--json` document counts
  `update-available 1, up-to-date 2, pinned 1, ignored 1, guess-only 1, source-gone 2, not-checked 2, failed
  0`. `update pinned --yes` ends `nothing updated: 1 pinned`, `update Questie --yes` ends `1 addon
  updated`, an unknown and an unmanaged name are exit 1 with their plain lines, and no name with no
  `--all`, or both, is exit 2. The link test refuses the update of a managed `OneFolder` whose folder is a
  junction made by hand (`Error::ForeignJunction`), and the entry stays at 1.1.0.
- 2026-10-22 — `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
