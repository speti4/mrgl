---
size: S
origin: done/066-github-source-looks-past-the-newest-release.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 078 — The window's cached view: the Forever build signal walks the releases like the resolve, and the last online answer is kept

## Context

`GitHub::cached_forever_build` (and any cache-only compare left in the window) takes the newest cached
release that has an asset for the flavor, as the resolve does since 066, instead of the channel's newest
release; and the window keeps the answer of its last online check (a source gone by a 404, a failed check,
the budget stop) across a redraw caused by a mark or a restart, until the next check replaces it. A
repository whose newest release lacks a Forever zip shows a Forever build of `no` and an update the
resolve would not install; and the window forgets what the check found as soon as a row is marked (010).
Done when: the fixture repository of 066 shows the Forever build of the release before the newest in the
window's offline list, and a flavor list test marks a row after a check that reported a source gone and still
shows it gone.

Decided 2026-10-22: first of the chain 078 → 079 on one branch, in wave 6.

## Docs that move

- `docs/development.md`: the `flavors` command's offline rule (the cached Forever build walks the releases,
  the last online answer kept) and the GitHub source's cached signal.

## Not in this change

- A new online request from the list; the CLI's `list` (its Forever build comes from the same core call and
  follows); the checks after an update and the title bar (079).

## Steps

1. [x] `sources/github.rs`: `cached_forever_build` walks the cached releases of the channel, newest first, to
       the first with an asset for the flavor, as the resolve does; any cache-only compare left in
       `mrgl-app/src/list.rs` follows the same walk
2. [x] The window keeps the answer of the last online check per addon (source gone, failed, the budget stop)
       across a redraw caused by a mark and across a restart, until the next check replaces it
3. [x] Tests: the fixture repository of 066 shows the Forever build of the release before the newest in the
       offline list; a flavor list test marks a row after a check that reported a source gone and still shows
       it gone; the stub tests of the source
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 078 → 079
- 2026-10-22 — core: `sources::github::forever_build_on(releases, channel, release_json)` walks the
  releases of `releases_on` newest first, as the resolve does since 066: `yes` at the first release whose
  `release.json` names Forever for a zip it has, `unknown` at the first with a Forever zip by its name only
  (the resolve's guess) or whose `release.json` is not known, a release without a Forever asset passed, and
  the newest release's own signal when none has one. `GitHub::cached_forever_build(spec, channel)` feeds it
  the cached list and the per-tag `release.json` cache (an unreadable one counts as none, as the resolve
  falls back to the names); the window's rows and the command line's `list` pass the entry's channel. The
  window's other cache-only compare, the rows' `update` and `check`, already came from `update::check` over
  the offline source (task 010), whose resolve walks the releases; nothing else compared the channel's
  newest release.
- 2026-10-22 — the kept answer: `mrgl_core::last_check` keeps, per addon, the last online check's answer of
  `source-gone`, `failed` or `rate-limited` in `<home>\<flavor-id>\last-check.json` (`format`
  `mrgl-last-check`, `schema` 1, the flavor id, and per sidecar name the source and version it was given
  for, the state, the release, the reason and the time). `actions::run` records every check's addons there
  (an addon's answer replaced by its next check, dropped for any other state; the addons a check did not
  cover keep theirs) and a failed write is a warning; `list::flavor_view` lays the kept answers over the
  offline check and under the run's own answer. Only these three states are kept because the offline check
  answers every other one from the cache, and after a mark (a pin, the channel) it answers with the mark
  applied, which a kept answer would hide. An answer holds while the sidecar entry keeps the source and the
  version it was given for, so an update or a reinstall, by the command line too, drops it; an ignored
  addon shows its ignore. A file of its own rather than the sidecar's entries: the sidecar is the install
  record the engine and the command line rewrite, and a corrupt sidecar rebuilds from the scan, while this
  is a network cache that any reader may drop; kept as a cache (a missing, unreadable or foreign file is no
  answer), it needs no migration and the sidecar's schema stays as it is. In the core, not the window's
  crate, since the window's crate has no JSON dependency of its own and the file sits next to the sidecar
  the core owns; `UpdateState` gained `Deserialize` for it.
- 2026-10-22 — tests: `forever_build_on` (unit: the walk on both channels, an unknown `release.json` on the
  way, no Forever asset at all, a Forever zip by name, no release); `tests/github_source.rs`
  `the_cached_forever_build_walks_past_the_newest_release_as_the_resolve_does` (066's repository: `yes`
  from the cache without a request after a Forever resolve, `unknown` on the prerelease channel's beta
  that has a Forever zip by name only, `unknown` once v1.9.0's cached `release.json` is removed);
  `tests/flavor_list.rs` `the_forever_build_offline_is_the_one_of_the_release_the_resolve_takes` (066's
  repository written into the fixture's stub folder); `tests/update_actions.rs`
  `a_source_gone_stays_on_its_row_across_a_mark_and_a_restart` and the budget test's kept rows; the
  `last_check` unit test (kept, replaced, dropped by another version, a mark, an ignore, another flavor's
  or a broken file).

## Deviations from the plan

- The kept answers live in the core (`mrgl_core::last_check`) rather than in the window's crate, and the
  window keeps three states only (source gone, failed, the budget stop), the ones the offline check cannot
  give again; see the Log.
- The core's `UpdateState` gained `Deserialize`, and `GitHub::cached_forever_build` takes the channel, so
  the command line's `list` passes the entry's channel too (its own lines are unchanged).
- No list compare besides the Forever build needed the walk: the rows' update already came from the
  offline `update::check` (task 010).

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 186, was 184; `tests/github_source.rs` 32,
  was 31; `tests/flavor_list.rs` 14, was 13; `tests/update_actions.rs` 9, was 8; `tests/cli.rs` 118
  unchanged; the Node tests through `ui_smoke`). On task 066's repository (a draft and a beta with Forever
  zips above a stable v2.0.0 built for classic only, v1.9.0 with the Forever zip below it), the window's
  offline list shows a Forever addon installed at 1.8.0 with Forever build `yes` and the update to 1.9.0
  (`v1.9.0`), where the channel's newest release had said `no`; `cached_forever_build` answers `yes` from
  the cache with no request. On the made-up Forever flavor of four addons, a check against a stub source
  whose `github:Example/Wrapped` answers 404 shows `Wrapped` as `source gone`; after a pin on another row,
  the prerelease channel and a pin on `Wrapped` itself, each mark's answer still shows it gone, and so does
  the list read again with no check (the restart), its line naming `no such repository, or it is private`;
  after the unpin, a check of `Wrapped` alone against a source that answers again shows it `up to date`,
  and the next list keeps no gone answer. After the budget stop, the list read again shows the two addons
  the check left as `not checked (rate limit)`.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` green; `python -I scripts/changelog.py --check` green after the regeneration. No
  manifest changed, no new crate.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
