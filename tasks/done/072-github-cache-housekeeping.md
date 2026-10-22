---
size: S
origin: done/042-github-source-pagination-rate-limit-and-per-tag-cache.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 072 — GitHub cache housekeeping: stale per-tag `release.json` files and the old single file

## Context

The per-tag `release.json` cache (042) drops the files of tags the cached release list no longer holds, and
the old `release-json.json` left in existing home folders is removed on the first resolve. The cache grows
by one file per release forever, and the old file is dead weight nobody reads.
Done when: a fixture cache with stale tags and the old file is cleaned on a resolve, and a current tag's
file stays.

Decided 2026-10-22: second of the chain 071 → 072 on one branch.

## Docs that move

- `docs/development.md`: the cache layout under `%LOCALAPPDATA%\mrgl\`, if the per-tag files are listed.

## Not in this change

- The zip cache under `cache\<addon>\<version>.zip` (its own housekeeping is not asked for); a size bound
  on the cache.

## Steps

1. [x] `sources/github.rs`: after a fresh release list is written, the `tag-*.json` files of tags the list
       no longer holds are removed; a `release-json.json` next to them is removed when present
2. [x] Tests against the stub: a fixture cache with stale tags and the old file is cleaned on a resolve,
       a current tag's file stays, an offline resolve (041) removes nothing
3. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; second of the chain 071 → 072
- 2026-10-22 — `sources/github.rs`: `GitHub::prune_release_json_cache(owner, repo, releases)` next to
  `release_json_cache_path`, called by `release_list` once the list's cache file is written, on a 200 with
  the fresh list and on a 304 with the cached one the API confirmed. It removes every `tag-*.json` under
  `release-json/` whose name is not `tag-<file_safe(tag)>.json` of a `tag_name` the list holds (a draft
  the list holds counts), only files, and the old `release-json.json` next to `releases.json`
  (`OLD_RELEASE_JSON_CACHE_FILE`, private). Every removal is best effort, as every cache write of the
  source. The file name moved into `release_json_cache_name(tag)`, which the cache path and the prune
  share. The offline branch (task 041) returns before the cache is written, so it removes nothing.
- 2026-10-22 — tests in `tests/github_source.rs`: `a_resolve_drops_the_cached_release_json_of_tags_the_list_no_longer_holds`
  plants the cache an older mrgl leaves (`release-json.json`, `tag-v1.0.0.json`, `tag-v0.9.0-beta.json`,
  `tag-v1.2.2.json` of a listed tag, a `notes.txt`) under `Questie/Questie` and resolves against the
  fixture list (`v1.3.0` draft, `v1.2.3`, `v1.2.2`), once fresh and once after a 304;
  `an_offline_resolve_removes_nothing_from_the_cache` plants the same files in a warm cache and resolves
  offline.

## Deviations from the plan

- The cleanup also runs on a 304, with the cached list the API confirmed: an existing home folder's first
  resolve is usually a 304, and the old `release-json.json` is to go on that first resolve.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green, `tests/github_source.rs` 26 with the two new
  ones. After the fresh resolve the repository's cache holds exactly `releases.json`,
  `release-json/tag-v1.2.3.json` (written by the resolve), `release-json/tag-v1.2.2.json` with its planted
  content and `release-json/notes.txt`; after the 304 resolve (3 requests in all, the third with
  `If-None-Match`) the same four, the current tag's file byte for byte as before. The offline resolve
  answers a stale pick and leaves all 7 files as they were. With the prune made a no-op, the first test
  fails and the offline one passes.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90
  check --all-targets --locked` green; `cargo check --workspace --all-targets --locked --target
  x86_64-apple-darwin` without a warning.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
