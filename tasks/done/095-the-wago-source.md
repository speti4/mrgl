---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M4 Sources
---

# 095 — The Wago source

## Context

`wago:<id>` resolves through the Wago Addons external API with the user's Bearer key (`MRGL_WAGO_KEY`,
093), the game version per flavor (`retail`, `classic`, `forever`, the progressing classic by its build), the
three recent releases mapped onto the channels, the download fetched with the same key and sent to Wago's
host only, the rate-limit headers feeding the request budget, the answer cached under `cache\wago\`, a 401
naming the key; the Forever signal is `yes` or `no` from a release for `forever`, and the keyless public data
endpoints (the game list, the slugs per game version) answer the signal and the id lookup without a key.
Decided 2026-10-22: the keyless data endpoints are used as they are; the answers are cached like GitHub's.
Why: the one source after GitHub whose API names the Forever flavor. Done when: `add wago:<id>` installs from
the stub for Forever and retail, a 401 stub names the key, the channels are tested over the three stabilities,
the check stops before the stub's rate limit, a list entry imports, and fixtures classify Forever `yes` and `no`.

Decided 2026-10-22: runs after 094 on the same branch; only the user's own key (D-002, 093).

## Docs that move

- `docs/development.md`: the sources layout (the Wago module, its cache folder, its fixtures), the keys section
- `README.md`: the source list, the `wago:<id>` spec and the `MRGL_WAGO_KEY` line in the usage text

## Not in this change

- The CurseForge source (096); the adopt report (097); the window's key field
- A per-source rate-limit stop of the update check (101)

## Steps

1. [x] `sources/wago.rs`: resolve by id with the Bearer key from the keys module of 093, the game version
       per flavor (the progressing classic by its build as the GitHub flavor keys do), the three stabilities
       onto the channels, the interface from the supported patches, the cache under `cache\wago\`
2. [x] The download fetched with the same key, sent to Wago's host only (a test); a 401 names the key; the
       rate-limit headers feed the request budget
3. [x] The Forever signal `yes` or `no` from a release for `forever`; the keyless data endpoints answer the
       signal and the id lookup without a key
4. [x] The source joins the router of 092; the list import and the update check take a `wago:` entry
5. [x] Fixtures (the Forever and the retail answer of one id through the stub's query mapping, a 401, a
       rate-limit answer, the zip behind the download link) and the tests of the done-when lines

## Log

- 2026-10-22 — promoted; runs after 094 on the same branch
- 2026-10-22 — `sources::wago`: `Wago::new(http, key, cache_dir)`, `from_env` (the key through
  `keys::of_process`) and `cached` (the window's offline view: no key, the cache alone, never a request to
  the external API); `addon(id, game_version)` (`WagoAddon` with its `WagoRelease`s, an answer read as an
  object or one under `data`), `slugs(game_version)`, `id_of_slug(slug, game_version)` and
  `forever_build(id, channel)`; the pure `game_version(target)` (`retail`, `classic`, `forever`; the
  progressing classic by the build's major, `titan` for `3.80`, `mop` without a build, a guess),
  `release_on(addon, channel)` (stable: `stable`, else the newest of the others; prerelease: the newest of
  the three, by `created_at`, the stabler one on a tie), `parse_addon` and `interface_of_patch`. The key
  goes with a request only where `keys::may_send` allows it (`https://addons.wago.io`), the external API and
  a download link there; the keyless data endpoints get none. Without the key, `Source::missing` says that
  `MRGL_WAGO_KEY` is not set and that `mrgl key set wago` stores it, and nothing is asked.
- 2026-10-22 — the pick: the version is the `label` without a leading `v`, the asset `<slug>-<label>.zip`,
  the interface the newest supported patch (`supported_<v>_patches`, else `supported_<v>_patch` or
  `supported_patches`), the page `addons.wago.io/addons/<slug>`, the reason `Wago lists a <stability>
  release for <v> (patch <p>)`, or on the stable channel without a stable release `Wago lists no stable
  release for <v>; the newest, the <stability> release, is taken`. `resolve_version` finds a version among
  the three recent releases. An answer without a release for the game version is the no-asset error, a 401
  names the key, a 403 or 429 with no request left names the rate limit.
- 2026-10-22 — the budget: every request to Wago's host counts; the limit is `x-ratelimit-limit` (60 until
  an answer says) and the requests left are the last `x-ratelimit-remaining` less the requests since.
  `RequestBudget` and `BudgetStop` gained `window` and `resets` (left out of the JSON), so the stop line
  says each source's limit in its own words: GitHub's line is unchanged, Wago's is `stopped the check before
  Wago's rate limit of 60 requests: <n> addons not checked, the limit resets after a short wait (Wago does
  not say when); run the check again then`.
- 2026-10-22 — the cache: `<home>/cache/wago/<id>.json` holds one answer per game version with
  `checked_at`; offline the cached answer stands with `stale`. The slug lists of the keyless data endpoint
  (`api/data/slugs?game_version=<v>`, `{"addons": {"<slug>": {"id": …}}}`) are read at most once a run and
  once a day and cached as `slugs-<v>.json`, slug to id. The Forever signal: from the cached `forever`
  answer (`yes` with a release on the channel, `no` without), else from the slug list of `forever` (`yes`
  when it names the id, `no` when it does not); `cached_forever_build` reads the cache alone, the pick asks
  the slug list once a day when nothing is cached.
- 2026-10-22 — the router holds the source in `Sources::from_env` and `Sources::cached`; the CLI's
  `forever build:` line says `Wago lists a release for forever` or `Wago lists no release for forever`.
- 2026-10-22 — fixtures: `tests/fixtures/sources/addons.wago.io/` with `api/external/addons/<id>@game_version=<v>.json`
  for `MrglWg01` (a Forever and a retail release, a stored zip behind each, written by `zips/make-zips.py`),
  `MrglWg02` (no release for Forever), `MrglWg03` (the three stabilities for retail, a beta and an alpha for
  Forever, a stable release for `mop`), `MrglWg04` (401, through a `.headers` file), `MrglWg05` (no request
  left, through a `.headers` file), and the keyless `api/data/slugs@game_version=forever.json`.
- 2026-10-22 — tests: `tests/wago_source.rs` (one URL's Forever and retail answers and the requests with
  and without the key, the channels over the three stabilities, the game type of the progressing classic,
  `resolve_version`, the Forever signal `yes` and `no` from the answers and from the keyless slug list,
  `id_of_slug`, no key in a cache file, the 401 and the run without a key, the key with the download and
  never off Wago's host, the budget from the headers, the offline answer and the window's view); core unit
  tests of `wago` (the channels, the game versions, an answer's releases and patches); `tests/cli.rs`
  `add_wago_installs_the_forever_and_the_retail_answer_of_one_url`, `add_wago_names_the_key_on_a_401`,
  `import_takes_a_wago_entry_and_update_stops_before_wagos_rate_limit`.
- 2026-10-22 — docs: `docs/development.md` (the layout's `sources::wago`, a Fixtures paragraph on the
  source, its cache, its budget and its fixtures, the keys section's hosts), `README.md` (the sources this
  build has, how a Wago pick is made, the Forever line, the import, an `add wago:<id>` line in the usage
  block), `tests/fixtures/README.md` (the Wago answers under `sources/`).

## Deviations from the plan

- The game list endpoint (`api/data/game`) is not read: the game version follows the installed build as the
  GitHub flavor keys do, which the task names as the rule, and the Forever signal and the id lookup come
  from the slug list; nothing needed the game list.
- Two end-to-end tests of 092 expected `this build has no Wago source` for a `wago:` spec
  (`add_of_a_source_this_build_lacks_names_what_is_missing`, `add_failures_are_plain_lines_with_exit_code_one`);
  they now expect the missing key, the line this build gives without `MRGL_WAGO_KEY`.
- The stop line before a rate limit had GitHub's window in its text; it now comes with the budget
  (`window`, `resets`), so the router's test source and the window's update-action test stub set the two
  fields. GitHub's line is the same as before.
- No live request to the external API: it answers 401 without a key, and no key was used; its answers are
  stubbed from the field list the planning round gathered.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 231, `tests/wago_source.rs` 7,
  `tests/wowinterface_source.rs` 9, `tests/cli.rs` 147, `tests/flavor_list.rs` 17, the other targets
  unchanged). `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked`, `cargo deny check` and `python -I scripts/changelog.py --check` green;
  `make-zips.py` rewrites every earlier zip byte for byte.
- 2026-10-22 — by hand, the debug `mrgl` with `MRGL_HOME` under a temporary folder and `MRGL_HTTP_STUB` at
  `tests/fixtures/sources/`: without `MRGL_WAGO_KEY`, `add wago:MrglWg01 --flavor forever --dry-run` says the
  key is not set (exit 1); with a made-up key it picks `2.0.0` for Forever (interface 16001, `forever build:
  yes`), `wago:MrglWg03` for retail takes the stable `3.0.0`, and `wago:MrglWg04` gives the 401 line naming
  `MRGL_WAGO_KEY` (exit 1); the cache folder held one file per addon and `slugs-forever.json`, none with the
  key in it. One read-only probe of the keyless data endpoints learned their shape; the external API was
  not asked. No game folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
