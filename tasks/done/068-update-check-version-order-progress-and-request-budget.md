---
size: M
origin: done/055-one-core-helper-for-resolve-download-and-install.md
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 068 — The update check: version order, progress, and a request budget without a token

## Context

`update::check` reports "installed is newer than the current release" as its own state instead of an update
(a deleted release, a channel switch); it reports progress per addon through a callback the CLI and the
window (010) share; and it stops with a clear line before a large flavor exhausts the 60 unauthenticated
requests an hour (042's reset time in the line). Today any other version counts as an update, a long check
is silent, and a long list without a token hits the rate limit part way.
Done when: fixture sidecars cover the newer-installed state, the progress lines, and a stubbed rate limit.

Decided 2026-10-22: its own branch; the progress callback is the handover to 010, which starts from this
branch's tip.

Decided 2026-10-22: the version order is the source's release list (newest first, drafts left out) where
it lists both the installed and the current version, else a lenient numeric compare (a leading `v`
dropped, the numbers before the first `-` or `+` compared as numbers, a suffix before no suffix on equal
numbers); when neither can tell, another version counts as an update, as before. The budget stops the
check when nothing is left (no request is sent that the API would refuse); a stop is a warning, not a
failure, so the exit code stays 0.

## Docs that move

- `docs/development.md`: the update states (the newer-installed one), the progress line, and the request
  budget without a token.
- `README.md` ("What it will do"), if the update check's lines are quoted there.

## Not in this change

- The window's update button and progress (010); a token prompt or store (the token comes from the
  environment as 042 reads it); the other sources (056).

## Steps

1. [x] `update.rs`: the version order (the release list's order where the installed version is a known tag,
       else a lenient numeric compare) and a new `UpdateState` for an installed version newer than the pick,
       with its own voice line
2. [x] A progress callback (`FnMut(Progress)` or a small trait) called once per addon, before and after its
       check; the CLI prints one line per addon through the voice, `--json` unchanged
3. [x] A request budget on the GitHub source without a token: the requests of this run are counted, the
       check stops before the limit with the reset time of 042 in one line, the remaining addons are reported
       as not checked
4. [x] Tests: fixture sidecars for the newer-installed state, the progress lines, and a stubbed rate limit
       (the stub answers the `x-ratelimit-*` headers)
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-22 — `sources.rs`: two default methods on `Source`, `listed_versions(spec)` (the versions the
  source lists, newest first, without a request; `None` by default) and `request_budget()` (`None` by
  default), and the `RequestBudget` struct (host, limit, sent, remaining, reset, hint).
  `sources/github.rs`: every API request of `release_list` (the first page and the page walk) goes
  through `GitHub::api_get`, which counts the answered requests and keeps the last
  `x-ratelimit-remaining` (a 429 counts as none left) and the reset time of `rate_limit_reset`; asset
  downloads are not counted. `request_budget` is `None` with a token, else `ANONYMOUS_LIMIT` (60) less the
  requests sent until an answer names the remaining count, then that count less the requests since.
  `listed_versions` reads the cached release list (no request), drafts left out.
- 2026-10-22 — `update.rs`: `UpdateState::InstalledNewer` (`installed-newer`, label `newer than the
  release`, line `newer than the current release <v>, left alone`, with `(<installed> is no longer
  listed)` when the cached list lacks the installed version) and `UpdateState::RateLimited`
  (`rate-limited`, label `not checked (rate limit)`); `lenient_order` is public. `check` takes
  `progress: &mut dyn FnMut(CheckProgress<'_>)`: `Checking { index, total, name, source }` before an addon
  the source is asked about, `Checked { index, total, addon }` after every addon, `Stopped { stop }` when
  the budget is used up. Before each addon that needs a request the budget is read; with nothing left the
  addons left are `rate-limited` and `UpdateCheck::stopped` carries the `BudgetStop` (host, limit, sent,
  reset, not_checked, hint) with its one line.
- 2026-10-22 — `mrgl-cli`: `update` prints `checking <name> against <source> (<i> of <n>)` through the
  voice before each asked addon, the data line as soon as the addon is checked (the lines are streamed,
  no longer printed after the whole check), and the stop as a warning; the summary counts both new
  states and treats `rate-limited` as needing a look; the JSON document adds `stopped` (`null`, or the
  `BudgetStop` with its `message`) and the two new keys of `counts`.
- 2026-10-22 — tests: the fixture sidecar of the update tests gains `Delisted` (1.5.0, a version
  `Example/Prerelease` does not list: the lenient order) and `Switched` (1.1.0-beta on the stable
  channel: the list order), both `installed-newer`; the existing update tests assert the progress lines;
  `update_stops_before_the_rate_limit_with_the_reset_time_and_counts_the_rest` stubs the first release
  list's answer with `X-RateLimit-Remaining: 0` and a reset time in a `.headers` file;
  `github_source.rs` covers the counting, the headers, a 429, the 60 anonymous requests, a token and the
  network; `update::tests` covers the lines, `lenient_order` and the stop line.

## Deviations from the plan

- The callback is `&mut dyn FnMut(CheckProgress<'_>)`, as `Engine::install_from_source` takes its
  `PickStep` callback, rather than a trait; `CheckProgress` borrows from the check, so it is `Send` and
  `Sync` whenever the check's data is.
- "Before and after its check": `Checking` is sent only for an addon the source is asked about (not for an
  ignored addon, one with no source to ask, or one left for the limit), `Checked` for every addon.
- Besides the newer-installed state, a second new state, `rate-limited`, reports the addons the stop left;
  `not-checked` keeps its meaning (no source to ask).
- The order by the release list compares versions, not tags: the sidecar records the version (the tag
  without its `v`), and so does the list.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 175, `tests/cli.rs` 115,
  `tests/github_source.rs` 26). `update --all --dry-run` over the fixture sidecar prints, among the
  progress lines `checking Beta against github:Example/Prerelease (1 of 12)` through `checking Switched
  against github:Example/Prerelease (11 of 12)`, the lines `Delisted  1.5.0  newer than the current
  release 1.0.0 (1.5.0 is no longer listed), left alone` and `Switched  1.1.0-beta  newer than the current
  release 1.0.0, left alone`, and the summary `dry run: 1 update available, 2 up to date, 2 newer than the
  release, 1 pinned, 1 ignored, 1 guess only, 2 source gone, 2 not checked; nothing written`; `update
  --all --yes` updates Questie only and leaves both sidecar entries as they were; the JSON document has
  `installed-newer: 2`, `rate-limited: 0` and `stopped: null`. With the first answer stubbed to
  `X-RateLimit-Remaining: 0`, the dry run checks `Alpha` (`2.0.0 -> 2.1.0  update available`), prints
  `stopped the check before GitHub's rate limit of 60 requests an hour without a token: 2 addons not
  checked, the limit resets at <local time> (local time); set MRGL_GITHUB_TOKEN to a token of your own for
  5000 requests an hour` (the murloc line of a warning before it without `--serious`), reports `Bravo` and
  `Delta` as `not checked (rate limit)` while the zip and the ignored addon keep their own states, sums up
  `dry run: 1 update available, 1 ignored, 1 not checked, 2 not checked (rate limit); nothing written`,
  exits 0, and asks for no release list after the first; the JSON document carries `stopped` with `sent: 1`,
  the reset, `not_checked: 2` and the line. `github_source.rs`: three counted requests leave 57, an answer
  with `X-RateLimit-Remaining: 7` sets 7 and one more request 6, a 429 sets 0, sixty requests without
  headers set 0, a token gives no budget, an offline request counts nothing; the listed versions of
  `Example/Prerelease` are `1.1.0-beta`, `1.0.0` from the cache without a request.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` and `cargo +1.90
  check --all-targets --locked` green; `python -I scripts/changelog.py --check` green after the
  regeneration.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
