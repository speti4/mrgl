---
size: S
origin: done/003-github-release-source-with-release-json.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 042 — GitHub source: pagination past 30 releases, the rate-limit reset time, `release.json` cached per tag

## Context

Three edges of the GitHub source (003): the release list follows the `Link` header past the first
page of 30 when the first page holds no stable release; a 403 or 429 hint names the time the rate limit
resets (`x-ratelimit-reset`) and the token variable; and a release's `release.json` is cached per tag next
to the release list, so a repeated resolve costs one conditional request, not two. A repository that
publishes nightly prereleases pushes the stable release off the first page, and a user without a token
hits the hourly limit on an `update --all` over a long list.
Done when: a fixture repository with 35 releases resolves the stable one on page two, the 403 hint carries
the reset time, and a second resolve of the same tag reads `release.json` from the cache (the stub's log
shows one request).

Decided 2026-10-22: first of the chain 042 → 046 → 041 on one branch, all three in `sources/github.rs`.

## Docs that move

- `docs/development.md` (the source and its cache under the home folder): the per-tag cache file and the
  page walk.

## Not in this change

- The asset download's error path and size check (046, next in the chain); the offline resolve (041);
  any other source.

## Steps

1. [x] The release list follows `Link: rel="next"` while no stable release was found, with a page bound
2. [x] A 403 or 429 answer renders the reset time from `x-ratelimit-reset` (local time) and names
       `MRGL_GITHUB_TOKEN` in the hint
3. [x] `release.json` cached per tag next to the release list under the home folder, read before the
       network on a repeated resolve
4. [x] Tests on the stub: 35 fixture releases with the stable one on page two; the 403 hint; one request on
       the second resolve of a tag
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first of
  the chain 042 → 046 → 041
- 2026-10-22 — `sources/github.rs`: `GitHub::releases` reads the first page as before and, while the pages
  read hold no release that is neither a draft nor a prerelease, follows the `Link` header's `rel="next"`
  URL with the API headers and the token, up to `MAX_PAGES` (5, 150 releases); a next link off
  `https://api.github.com/` is not followed, and a page that answers anything but 200 is the status error
  of that page's URL. The cache file keeps every page read as one list with the first page's `ETag`, so a
  304 answers from the whole walked list. Past the bound the current release is the newest prerelease, the
  rule for a repository without a stable release.
- 2026-10-22 — `status_error`: a 429, or a 403 with `x-ratelimit-remaining: 0`, reads `the rate limit is
  used up until <YYYY-MM-DD HH:MM> (local time); set MRGL_GITHUB_TOKEN to a token of your own for 5000
  requests an hour`, the time from `x-ratelimit-reset` or, without it, now plus `retry-after`; without
  either the line drops the time. `sources::local_time_text` renders Unix seconds in the machine's time
  zone through the `backup::clock` conversion of task 013 (no date crate).
- 2026-10-22 — the `release.json` cache is per tag: `<home>/cache/github/<owner>/<repo>/release-json/
  tag-<tag>.json` (the tag through `state::file_safe`) holds the tag, the asset's URL and size and the raw
  document; `GitHub::release_json` answers from it without a request when all three match, else downloads
  and writes it. `cached_forever_build` reads the current release's file from the same folder; the single
  `release-json.json` of task 011 and its constant `RELEASE_JSON_CACHE_FILE` are gone
  (`RELEASE_JSON_CACHE_DIR` replaces it).
- 2026-10-22 — `sources.rs`, the folder stub: a `page` above 1 in the query answers from
  `<path>.page-<n>` (then `.json`), and a `<answer>.headers` file adds its `Name: value` lines to the
  answer's headers, a `Status: <code>` line setting the status (never turned into a 304).
- 2026-10-22 — fixture `releases/api.github.com/repos/Example/Nightly/`: `releases.json` with 30 nightly
  prereleases and the `Link` header in `releases.json.headers`, `releases.page-2.json` with the stable
  `v1.0.0` above two betas and two older stable releases, and that release's `release.json` naming
  `forever`; the Forever classification of every fixture repository lists it as `yes`.

## Deviations from the plan

- The page walk follows the list, not the channel: it stops at the first stable release on both channels,
  so the cache holds one list that serves both.
- The per-tag cache replaces the single `release-json.json` of task 011 instead of sitting next to it; a
  cache written by an earlier build is ignored and the file is left behind (the cache can be deleted any
  time).
- The cached `release.json` is checked against the asset's URL and size as well as the tag, so a
  re-uploaded `release.json` is downloaded again.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green. `tests/github_source.rs`:
  `a_stable_release_on_page_two_is_found_through_the_link_header` resolves `github:Example/Nightly` for
  Forever to `v1.0.0` (`Nightly-v1.0.0-forever.zip`, interface 16001, not a guess) with exactly three
  requests (page one, `…&page=2`, the release's `release.json`), the second page carrying `Authorization:
  Bearer secret`, `releases()` returning 35 releases, and the prerelease channel picking
  `v1.1.0-nightly.30`; `the_page_walk_is_cached_and_a_second_resolve_asks_once` finds 35 releases in the
  cache file and one request (page one, conditional) on the second resolve;
  `the_page_walk_stops_at_the_bound_and_off_the_api_host` reads exactly `MAX_PAGES` pages of a seven-page
  chain of prereleases and picks the newest, and does not follow a `Link` to another host;
  `the_rate_limit_hint_names_the_reset_time_and_the_token` gets `GitHub answered 403 for
  https://api.github.com/repos/o/r/releases?per_page=30: the rate limit is used up until <local time of
  1792598400> (local time); set MRGL_GITHUB_TOKEN to a token of your own for 5000 requests an hour`, a 429
  with `Retry-After: 60` names a reset time and the token, and a 403 with requests left stays `access
  denied`; `a_second_resolve_of_a_tag_reads_release_json_from_the_cache` counts two requests on the first
  resolve of `github:Questie/Questie`, one (the conditional list request) on the second, and a download
  again once the cached file names another asset URL. `sources.rs`
  `the_folder_stub_serves_pages_and_a_headers_file` covers the stub's page files, the `Link` and `Status`
  lines and the 304 rule.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
