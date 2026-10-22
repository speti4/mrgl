---
size: S
origin: done/028-lists-as-discovery-and-forever-starter-list.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 066 — The GitHub source looks past the newest release for the flavor's asset and for a named version

## Context

The resolve takes the newest release that has an asset for the flavor, not only the newest release; and
`import --exact` (057) walks further pages for a named version instead of stopping where 042 stops. Grid2's
newest release is on the classic line with no Forever zip, so it resolves to nothing on Forever and stays
out of the starter list (028); an older listed version beyond page one reads as not offered.
Done when: a fixture repository whose newest release lacks a Forever asset resolves the one before it, and a
named version on page two installs; both against the stub.

Decided 2026-10-22: third of the chain 064 → 065 → 066 → 067 on one branch.

## Docs that move

- `docs/development.md`: the GitHub source's pick rule (the newest release with an asset for the flavor) and
  the page walk for a named version.

## Not in this change

- The other sources (056); a bound change on the page walk beyond what a named version needs; the starter
  list's entries (028, rechecked after the launch in 008).

## Steps

1. [x] `sources/github.rs`: the resolve walks the cached release list from the newest release down to the
       first one with a pick for the flavor (drafts and the channel rule unchanged); the detail of
       `NoAssetForFlavor` says how many releases were looked at
2. [x] `resolve_version` walks further pages for a named version the first pages lack, within the page bound
       of 042, and only then answers `VersionNotOffered`
3. [x] Tests against the stub: a fixture repository whose newest release has no Forever asset resolves the
       one before it; a named version on page two installs through `import --exact`
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; third of the chain 064 → 065 → 066 → 067
- 2026-10-22 — `sources/github.rs`: `releases_on(releases, channel)` lists the releases a channel may take,
  newest first: on the stable channel the releases that are neither a draft nor a prerelease (every
  release that is not a draft when there is none of them), on the prerelease channel every release that is
  not a draft; `current_release_on` is its first. `GitHub::resolve` walks them through `pick_of` down to
  the first with a pick: a `NoAssetForFlavor` is passed, any other error (an unreadable list, a failed
  `release.json` download) stands. A pick past newer releases says so at the end of its reason (`…; the
  newest release with a forever asset, 1 newer release without one`); when none has one, the error is the
  newest release's with `; looked at <n> release(s)` (`, none has one` for more than one) at the end of its
  detail. The pick's Forever signal is the picked release's.
- 2026-10-22 — `GitHub::resolve_version`: a version the release list lacks is looked for on the pages after
  it (`further_pages`): `page=<n>` of the API from the first page the list does not hold, until the version
  turns up, a page is short (the last one) or `MAX_PAGES` pages are held in all; a list that is not a whole
  number of pages is complete, and nothing is asked. Only then does it answer `VersionNotOffered`. The
  further pages are not cached, and a list read offline (stale) is not extended.
- 2026-10-22 — tests: `tests/github_source.rs`
  `a_newest_release_without_a_forever_asset_is_passed_for_the_one_before` (a draft and a beta with Forever
  zips above a classic-only v2.0.0 and a Forever v1.9.0: the stable Forever pick is v1.9.0 with its
  `release.json` and the note, asking the list and two `release.json` files only; classic era stays on
  v2.0.0 without a note; the prerelease channel takes the beta; cata finds nothing: `looked at 3 releases,
  none has one`), `a_named_version_past_the_first_page_is_looked_for_on_the_next` (30 releases on page one,
  v1.0.0 on page two: found with one more request; a version on page one asks once; a missing one asks both
  pages and reads `no release 0.1.0 in github:o/r`; the current release asks for no further page) and
  `the_walk_for_a_named_version_stops_at_the_page_bound` (six full pages: a version on page five is found
  with five requests, one on page six is not offered after five); `tests/cli.rs`
  `add_takes_the_newest_release_with_a_forever_asset` and `import_exact_installs_a_listed_version_from_page_two`
  over a stub copy with `Example/Behind` and `Example/Paged`. The four expectations of a no-asset error
  (`github::tests`, `tests/github_source.rs` three, `tests/cli.rs` one) gain `; looked at 1 release`.
- 2026-10-22 — docs: `docs/development.md` (the source's pick rule in the layout, the release list's walk,
  the `--exact` lookup) and the module doc of `sources::github`.

## Deviations from the plan

- The further pages for a named version are asked by `page=<n>` instead of the `Link` header: a release
  list answered by a 304 from the cache carries no next link, and the page number follows from the pages
  the list holds. They are not written to the cache, so a later named version past the first pages asks
  again.
- The walk of the current release covers every release the list holds, with no bound of its own: each
  release passed costs its `release.json` download once (cached per tag, task 042), and the asset
  downloads do not count against the API rate limit.

## Verification

- 2026-10-22 — against a stub copy with `Example/Behind` (v2.0.0 with a classic zip only, v1.9.0 with a
  Forever zip): `mrgl --serious add github:Example/Behind --flavor <temporary _forever_> --dry-run` prints
  `pick for forever, a guess: github:Example/Behind v1.9.0 (Behind-v1.9.0-forever.zip, flavor forever): no
  release.json names forever; the asset name ends in -forever, a guess; the newest release with a forever
  asset, 1 newer release without one`; `add github:Example/MainlineOnly` ends with `(release.json names
  mainline, standard; assets: Only-v4.0.0.zip, release.json; looked at 1 release)`, exit 1. `mrgl import
  paged.json --exact` with `github:Example/Paged` at 1.0.0 (page two behind 30 releases of 2.0.x) prints
  `Questie  installed 1.0.0 from github:Example/Paged`, exit 0.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (every suite green: core unit 176, `tests/github_source.rs` 27, `tests/cli.rs` 116), `cargo
  +1.90 check --all-targets --locked` and `python -I scripts/changelog.py --check` green before the close;
  the changelog regenerated with the close.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
