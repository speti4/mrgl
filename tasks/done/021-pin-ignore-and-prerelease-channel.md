---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 021 — Pin, ignore and the pre-release channel per addon; CLI `pin`, `unpin`, `ignore` and `unignore`

## Context

The sidecar entry of an addon gains `pinned` (a version, or none), `ignored` (a flag) and `channel`
(`stable` or `prerelease`, the GitHub `prerelease` flag). `update --all` reports a pinned addon (`pinned
at v1.2, v1.3 available`), counts an ignored one (`3 skipped`) and touches neither; a prerelease channel
lets one addon follow beta builds. CLI `pin <name> [version]`, `unpin`, `ignore` and `unignore`; the
window's row actions belong to task 010. An update that breaks an addon is the daily case of the first
weeks; every other manager has pin and ignore, and `update --all` is not usable without them.
Done when: a fixture sidecar with a pinned, an ignored and a prerelease addon produces the documented
report and summary counts, and `--dry-run` writes nothing.

Decided 2026-10-21: the three fields are optional in the sidecar schema (absent means unpinned, not
ignored, stable), so an existing `state.json` reads unchanged; the GitHub source honors the channel when
it picks the current release (a prerelease only on the `prerelease` channel, as before otherwise); the
`update --all` report itself is 006, this task ships the fields, the four commands, `list`'s marks and
the core query the update check calls (`pinned`, `ignored`, `channel` per entry).

## Docs that move

- `README.md`: the four usage lines.
- `docs/development.md` ("Layout"): the sidecar fields and the channel in the source's pick.
- `tasks/decisions/D-003-state-sidecar-under-localappdata.md`: none (the sidecar's schema version does
  not change; the fields are optional).

## Not in this change

- `update --all` and its report (006); the rollback that pins (022); the window's row actions (010).

## Steps

1. [x] `state.rs`: `pinned`, `ignored`, `channel` on the entry, optional in the JSON, with a round-trip test
2. [x] The GitHub source picks a prerelease when the target's channel says so (the current-release rule
       takes the channel)
3. [x] `mrgl-cli`: `pin <name> [version]` (no version pins the installed one), `unpin`, `ignore`,
       `unignore`, each on a managed addon of the flavor, the voice lines, `--json`; `list` marks a
       pinned, an ignored and a prerelease addon
4. [x] Tests: a fixture sidecar with the three cases, the four commands end to end, an unmanaged name
       refused with a clear error
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `sources.rs`: `Channel` (`stable`, the default, and `prerelease`; serialized in lower case,
  parsed case-insensitively, `beta` read as `prerelease`); `Target` carries `channel` (`Target::new` and
  `Target::from_flavor` give `stable`, `Target::with_channel` sets an addon's own). `sources::github`:
  `current_release_on(releases, channel)` is the rule, `current_release` stays as its stable form; on the
  prerelease channel the newest release that is not a draft is current, prerelease or not;
  `GitHub::resolve` reads the target's channel.
- 2026-10-21 — `state.rs`: `Entry` gains `pinned: Option<String>`, `ignored: bool` and `channel: Channel`,
  appended after `installed_at`, each `#[serde(default)]` and left out of the file while unset, so the
  schema stays 1 and a sidecar written before reads unchanged. The queries `Entry::pinned`, `ignored` and
  `channel` are what the update check of 006 calls; the setters `pin(version)` (none or empty pins the
  installed version), `unpin`, `ignore`, `unignore` and `set_channel` return whether the entry changed.
  `State::find` and `find_mut` take an addon's name or one of its folders (a `list` line shows folders).
- 2026-10-21 — `install.rs`: `Engine::pin`, `unpin`, `ignore`, `unignore` and `set_channel` read the
  sidecar, change one entry, write it only when it changed, and return a `Marked` (name, installed version,
  the three marks, `changed`, warnings); a name the sidecar does not know is `AddonNotFound`, or the new
  `Error::NotManagedToMark` when the folder exists. `Engine::commit` keeps the three marks of the entry it
  replaces, with a warning when the pin names another version than the one just installed.
- 2026-10-21 — `mrgl-cli`: `pin <name> [version]`, `unpin`, `ignore`, `unignore` and
  `channel <name> <stable|prerelease>`, each with `--flavor` and `--json` (one document: `kind`, `flavor`,
  `addon` with `name`, `version`, `pinned`, `ignored`, `channel`, then `changed` and `warnings`); a change
  is a success line, a mark that is set already is a nothing-to-do line and writes nothing. `list` reads the
  flavor's sidecar: one helper, `marks`, gives an addon's line suffix (`pinned at 1.2.0`, `ignored`,
  `prerelease`) and its JSON fields (`pinned`, `ignored`, `channel`; `channel` is null for an unmanaged
  addon), and the summary counts each marked addon once (`, 1 pinned, 1 ignored, 1 on the prerelease
  channel`).
- 2026-10-21 — README: the usage lines; docs/development.md, "Layout": the sidecar fields, the channel in
  the GitHub pick and the new commands.

## Deviations from the plan

- A fifth command, `channel <name> <stable|prerelease>`: without it the channel field could be set only by
  editing `state.json` by hand. It goes through the same core path as the other four.
- None of the five commands has a `--dry-run`: each changes one field of the sidecar, says what it did,
  writes nothing when the mark is set already, and is undone by its counterpart. The `--dry-run` of the
  Context belongs to the `update --all` report of 006.
- A reinstall (`add <zip>` over a managed addon) keeps the marks, which the plan did not name: they are the
  user's word about the addon, not about one version.
- The `add github:` resolve still runs on the stable channel; passing a managed addon's channel to the
  `Target` belongs to the install from a pick (040) and the update check (006).

## Verification

- 2026-10-21 — `cargo test --locked`: every test green. `state.rs`: an entry pinned, one ignored, one on the
  prerelease channel and one plain round-trip through `state.json` unchanged, the unset fields absent from
  the file; a sidecar written without the three fields reads with no warning as unpinned, not ignored,
  stable; the setters report whether they changed the entry; `find` takes a name or a folder.
  `sources::github`: the stable channel skips a newer prerelease, the prerelease channel takes it, a newer
  stable release wins on either channel, drafts never count; against the fixture `Example/Prerelease`, the
  prerelease channel resolves `v1.1.0-beta` (`Pre-v1.1.0-beta-forever.zip`) and the stable one `v1.0.0`.
  `tests/install.rs`: the five engine calls round-trip through the sidecar (the file byte-equal to the
  start once every mark is undone, a repeated mark reported unchanged), a reinstall keeps the marks and
  warns that the pin at 1.0.0 stays while 1.1.0 is installed, and an unknown or unmanaged name is an error
  that writes no sidecar.
- 2026-10-21 — end to end through the exe (`crates/mrgl-cli/tests/cli.rs`): a hand-written fixture sidecar
  with a pinned (`1.2.0` while `1.3.0` is installed), an ignored, a prerelease and a plain entry next to a
  folder installed by hand lists as `Beta  0.9.0-beta  16001  prerelease`, `ByHand  0.1  16001`,
  `Ignored  2.0.0  16001  ignored`, `Pinned  1.3.0  16001  pinned at 1.2.0`, `Plain  1.0.0  16001` and the
  summary `5 addons in _forever_ (wowf 1.60.1.70245), 1 pinned, 1 ignored, 1 on the prerelease channel`;
  `--json` carries the three fields per addon; `list` leaves the sidecar untouched. Eleven steps of `pin`,
  `unpin`, `ignore`, `unignore` and `channel` on two installed zips print their documented lines and leave
  `state.json` byte-equal to the start; `--json pin` is one document; an unknown name exits 1 with
  `addon not found: Nope (flavor forever)`, a folder installed by hand exits 1 with the not-managed line
  for each command and no sidecar is written; `channel ByHand nightly` is a usage error (exit 2). That
  `add --dry-run` writes nothing stays covered by the test of task 004.
- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90
  check --all-targets --locked` and `python -I scripts/changelog.py --check`: all green. No manifest
  changed, no new crate. No round on a real game folder: these commands write only the sidecar under the
  mrgl home.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
