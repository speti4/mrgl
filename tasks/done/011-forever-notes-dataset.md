---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M2 Window MVP
---

# 011 — Forever notes dataset: dead, crippled or viable, from the research

## Context

Two signals on an addon's row in the window and in `mrgl list`: derived, whether the addon's source
publishes a `forever` build (`release.json`: yes, no or unknown); and curated, a small JSON data file in
the repo for the crippled cases only (a build exists, but the hidden combat values or the missing combat
log leave it half-working), one line per addon with a reason, a `Verified:` date and a source, extended
through pull requests. Forever hides combat values and removes the combat log; a user should see why an
addon will not work before installing it.
Done when: the derived signal covers every fixture `release.json`, the curated file carries the research
page's crippled addons with dates, and a stale entry is visible from its date.

Decided 2026-10-21: the Forever status is derived first, curated second. The derived signal is read from
the release list the source already fetches (a `release.json` with a `forever` flavor: yes; one without:
no; no `release.json` or a pick by name: unknown) and shown by `add --dry-run` and, for a managed addon,
by `list`; the curated file is `data/forever-notes.json` (one entry per addon keyed by the source spec:
reason, `verified` date, source URL), read at build time into the core, and an entry older than a
documented age is marked stale in the output. The window's data contract in `mock-data.js` (`forever:
{ build, note }`) already names the two signals and does not change.

## Docs that move

- `docs/development.md` ("Layout", "Fixtures"): the `data/` file, its schema and how to add an entry.
- `CONTRIBUTING.md`: a line on contributing a Forever note (reason, date, source) through a pull request.
- `README.md`: what the Forever note on a row means.

## Not in this change

- The window's row (017's sketch shows the made-up data until the shell binds the core, 009); a signal
  for other flavors; automatic staleness checks in CI.

## Steps

1. [x] The derived signal in the source: `forever` build yes, no or unknown from the release list, on the
       pick and for a managed addon's source
2. [x] `data/forever-notes.json` with the research page's crippled addons (reason, `verified`, source) and
       a core reader with the stale rule and a schema check test
3. [x] `mrgl-cli`: the note and the build signal on `add --dry-run` and in `list` (plain and `--json`),
       through the voice
4. [x] Tests: every fixture `release.json` classified, the curated file parsed, a stale entry marked
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `sources::github::forever_build(release, release_json)`: `yes` when the `release.json` names
  `forever` or `camelot` (any case) for a zip the release has, `unknown` without a readable `release.json` or
  when only an asset name ends in `-forever` / `-camelot` (a pick by name), `no` otherwise. `Pick` gains
  `forever_build` as its last field, set on every pick whatever the target flavor. `GitHub::release_json`
  caches the parsed file with its release tag as `cache/github/<owner>/<repo>/release-json.json` (the
  release list's write-and-rename helper made generic), and `GitHub::cached_forever_build(spec)` answers from
  the two cache files alone, never a request: `unknown` without them, for a cached `release.json` of another
  release, or for a current release without one.
- 2026-10-21 — `mrgl_core::forever`: `ForeverBuild`, the curated file compiled in (`CURATED_JSON`,
  `include_str!` of `data/forever-notes.json`, parsed once into `CuratedNotes` with `deny_unknown_fields`),
  `CuratedNotes::problems` (the schema check), `ForeverStatus { build, note, kind, verified, source, stale }`
  and `STALE_AFTER_DAYS` = 90, the day arithmetic on the civil-date helpers of `backup::clock`. Decided
  2026-10-21: a note is stale when its `verified` date is more than 90 days old, a season of patches; an
  unreadable date counts as stale.
- 2026-10-21 — the curated file (schema 1) has `kinds` (Forever's restrictions, each with a reason, a date
  and a source) and `addons` (one note per addon keyed by the canonical source spec, naming its kind). It
  starts with the two restrictions the research describes in kind: `secret-combat-values` (the addon
  restrictions of Midnight, secret values included, are active in Forever; source: the Patch 1.60.1 API
  changes page the research lists) and `combat-log-events` (an addon that decides from combat log events or
  another player's combat state hits the restrictions, display-only use works; source: the community addon
  status page the research lists), both verified against their sources on 2026-10-21. `addons` is empty: see
  Deviations.
- 2026-10-21 — `mrgl-cli`: `add --dry-run --flavor forever` prints `forever build: <yes|no|unknown> (<why>)`
  after the pick, with `; note: <reason> (verified <date>[, stale: older than 90 days], <source>)` when a
  note applies (a success for `yes` without a note, a warning otherwise), through `forever_message`; the
  `add` JSON document gets `forever` (`null` for another flavor). `list` on Forever reads the sidecar
  (`forever_statuses`) and marks each managed addon's primary folder through one helper, `mark_forever`: the
  line gets `  forever:<build>` with `,note` and `,stale`, the addon's JSON object gets `forever` (`null` for
  an unmanaged addon); a curated note is a warning line `forever note on <folder>: …`. `scan` and the other
  flavors' listings are unchanged.

## Deviations from the plan

- The research page (2026-10-21) names no crippled addon: it describes Forever's restrictions in kind and
  lists the sources. The curated file carries only what the research names, so its `addons` map starts
  empty and the file starts with the two restrictions the research describes, under `kinds`; every future
  note names one of them. The stale rule and the note output are tested on a synthetic file in the unit
  tests.
- The sources describe the combat log as restricted for decisions, not removed: the `combat-log-events`
  kind says so instead of "the combat log is removed".
- The Forever line of `add` prints on a dry run only: the run without `--dry-run` is task 040's install
  path; the JSON document carries the `forever` field in both cases.
- `list` marks the primary folder of a managed addon only (the first of the sidecar entry's folders), so a
  multi-folder addon carries one mark; a zip in hand reads `unknown`.
- The JSON object is `forever: { build, note, kind, verified, source, stale }`: the window's `{ build, note }`
  with the note's kind, date and source and the stale flag next to them.

## Verification

- 2026-10-21 — `cargo test --locked`: every test green. `tests/github_source.rs` classifies every fixture
  repository (the test lists the fixture folders, so a new one has to be classified there): Questie,
  Prerelease (its stable v1.0.0) and BetaOnly `yes`, MainlineOnly and NoMetadata `no`, NoReleaseJson and
  Empty `unknown`, and every pick carries the same value for Forever and for retail; after three resolves
  into a temporary cache, `cached_forever_build` answers `yes`, `no` and `unknown` with no further request,
  and `unknown` once the cached `release.json` names another tag or is not JSON. The unit tests cover the
  `camelot` synonym, a `release.json` for mainline next to a `-forever` zip (`unknown`) and an entry whose
  zip the release lacks (`no`). `forever::tests` parses `data/forever-notes.json` with no schema problem,
  marks a synthetic note verified 2026-06-01 stale on 2026-10-21 and one of 2026-10-01 fresh (the boundary:
  exactly 90 days is fresh, 91 stale), and names every problem of a broken file (schema number, kind id,
  reason, http source, non-canonical key, unknown kind, future date, 2026-02-30, an unknown field).
- 2026-10-21 — end to end in `tests/cli.rs`: `mrgl --serious add github:Questie/Questie --flavor forever
  --dry-run` prints `forever build: yes (release.json names forever)` between the pick and `dry run: nothing
  installed`; NoReleaseJson prints `forever build: unknown (no release.json says)`; the JSON has
  `pick.forever_build` and `forever.build` `yes`, a retail dry run of MainlineOnly `pick.forever_build` `no`,
  `forever` `null` and no Forever line. A zip installed into a temporary `_forever_` with its sidecar entry
  pointed at `github:Questie/Questie` lists as `forever:unknown` before any `add` and `forever:yes` after a
  dry run filled the cache, with the `forever` object in the JSON; a retail listing has no `forever` field.
  The CLI's unit tests check the line text with a stale note and the `,note,stale` marks.
- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check`: all green. No new crate.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
