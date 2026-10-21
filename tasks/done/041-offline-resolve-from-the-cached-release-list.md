---
size: S
origin: done/003-github-release-source-with-release-json.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 041 — Offline resolve from the cached release list, with a warning

## Context

When the GitHub releases API cannot be reached (no network, a timeout) and the repository's release
list is in the cache of 003, the source resolves from the cached list and the voice warns that the pick
may be stale (the cache's date in the line); a cache miss stays the network error it is today. A resolve
on a flaky connection should not fail when yesterday's answer is at hand; `update --all` (006) over
twenty addons wants the same.
Done when: with the stub returning a network error and a warm cache, `add --dry-run` reports the pick with
the warning, and with a cold cache the network error.

Decided 2026-10-22: third of the chain 042 → 046 → 041; the per-tag `release.json` cache of 042 serves
the offline resolve too, so a stale pick needs no network at all.

## Docs that move

- `docs/development.md` (the source and its cache): the offline rule and the warning line.

## Not in this change

- An offline download (the asset is never cached before install); the window's offline state (009).

## Steps

1. [x] A network error with a warm release-list cache resolves from the cache; the pick carries a stale
       flag with the cache's date; a cold cache keeps the network error
2. [x] The voice's warning line on `add --dry-run`, `add` and the update check, plain and `--json`
3. [x] Tests on the stub: warm cache plus network error, cold cache plus network error, a warm cache with
       a working network (no warning)
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), third
  of the chain 042 → 046 → 041
- 2026-10-22 — `sources.rs`: `Pick` gains `stale: Option<Stale>` (serialized, `null` for a fresh pick),
  `Stale { checked_at: Option<String>, reason: String }` (the cache's date in RFC 3339 UTC and the network
  error's plain line), and `Pick::stale_warning()`, the warning line `offline: <source> <tag> is from the
  release list cached <YYYY-MM-DD HH:MM local>, the pick may be stale (<network error>)`, `cached at an
  unknown time` when the cache has no date. `unix_from_rfc3339` reads mrgl's own RFC 3339 back for the
  local rendering. The folder stub answers every request with `Error::Network` when its folder does not
  exist.
- 2026-10-22 — `sources/github.rs`: the release list (`release_list`, behind the unchanged public
  `releases`) answers a network error on its first request from the cached list with a `Stale`; any other
  error, a status and a cold cache stay as they were. The cache file keeps `checked_at`, written on a 200
  and rewritten on a 304, so the date is the last time the API confirmed the list. `resolve` puts the
  `Stale` on the pick; the release's `release.json` comes from the per-tag cache of 042, so a warm cache
  resolves with no request answered.
- 2026-10-22 — `mrgl-cli`: `add --dry-run` and `add` (`install_from_source`) push `pick.stale_warning()`
  to the warnings, printed as a warning message and listed in the JSON `warnings`; the JSON `pick` carries
  `stale`.

## Deviations from the plan

- The update check of step 2 is task 006, on another branch: the flag and its line are on the `Pick`
  (`stale`, `Pick::stale_warning`) for it to print; this change wires `add --dry-run` and `add`.
- `import` resolves through `list_file::import` and does not print the line yet (a follow-up, with the
  install helper of 055).
- An `add` offline gets as far as the sidecar check: a release installed already is reported with the
  warning, a new one fails on the asset download's network error (no offline download, as planned).

## Verification

- 2026-10-22 — `cargo test --locked`: every test green. `tests/github_source.rs`:
  `a_network_error_with_a_warm_cache_resolves_stale` resolves `github:Questie/Questie` online (no `stale`,
  no warning, `checked_at` in the cache file), then over a stub folder that does not exist: the same pick
  with `stale.checked_at` equal to the cache's date and `stale.reason` `cannot reach
  https://api.github.com/repos/Questie/Questie/releases?per_page=30: offline, the stub folder is missing`,
  one request in all (the failed list request), the warning line with the local time, `stale` in the
  JSON (and `null` for the fresh pick), and `cached at an unknown time` for a cache without the date;
  `a_network_error_with_a_cold_cache_stays_the_error` gets `Error::Network` with that line, with and
  without a cache folder; `a_conditional_answer_refreshes_the_cache_date` sees a 304 replace an old
  `checked_at` with no warning. `tests/cli.rs`: `add_dry_run_offline_answers_from_a_warm_cache_with_a_warning`
  runs `mrgl add github:Questie/Questie --flavor forever --dry-run` cold and offline (exit 1, the network
  line), online (no `offline` anywhere), offline again (`resolving …`, the warning line, the pick line, `dry
  run: nothing installed`), and with `--json` (`pick.stale` with the reason and a date, the line as the one
  entry of `warnings`; online `stale: null` and no warnings);
  `add_offline_finds_the_installed_release_from_the_cache_with_a_warning` installs online, then offline
  gets the warning and `Questie 1.2.3 is installed already in _forever_ from github:Questie/Questie,
  nothing to do`, and the same in the JSON (`already_installed: true`, `pick.stale`, the warning listed).
  `sources.rs` unit tests cover the missing stub folder and the RFC 3339 reader.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
