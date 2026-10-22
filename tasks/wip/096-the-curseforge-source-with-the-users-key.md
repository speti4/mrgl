---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M4 Sources
---

# 096 — The CurseForge source with the user's key

## Context

`curse:<id>` resolves through the CurseForge Core API with the user's key (`MRGL_CURSEFORGE_KEY`, 093):
the mod and its files per game version type (Forever included), the release types mapped onto the channels,
the download carrying the key too; a mod with distribution off fails with a line naming its page, and a zip
in hand installs with the source recorded; no API answer is cached, as the terms require, so the Forever
signal in `list` comes from what the sidecar recorded at install, else `unknown`. The code and the stubs land
before the key; the manual round and the type ids wait for it. Decided 2026-10-22: only the user's own key
(D-002, 093); never a cache of CurseForge answers. Why: the retail and classic folders are mostly CurseForge.
Done when: `add curse:<id>` installs from the stub with a key and names the key without one, a
distribution-off mod names its page and installs from a zip, a test asserts the key reaches the two CurseForge
hosts only and never a cache or the sidecar, a list entry imports, and a fixture classifies the Forever build.

Decided 2026-10-22: the code and the stubs are built now, on the branch of the source follow-ups (101 to
106), without the key; the task stays open after that change lands, until the key of 007 allows the manual
round and the re-read of the game version type ids. Also here: `Source::folders` from a file's `modules` and
the adopt lines (097) with a `curse:` source that can be asked.

## Docs that move

- `docs/development.md`: the sources layout (the CurseForge module, no cache folder, its fixtures), the keys
  section, the sidecar's recorded Forever signal
- `README.md`: the source list, `curse:<id>`, `MRGL_CURSEFORGE_KEY`, the distribution-off line

## Not in this change

- The manual round and the confirmation of the game version type ids (they wait for the key of 007)
- The window's key field; fingerprint matching

## Steps

1. [x] `sources/curseforge.rs`: the mod and its files per game version type with the key from the keys module,
       the release types onto the channels, the download with the key, the key sent to the two CurseForge hosts
       only (a test), no cache of any answer, `listed_versions` none
2. [x] A mod with distribution off fails with a line naming its page; a zip in hand installs with the source
       recorded; the sidecar records the Forever signal at install (an optional field, the schema unchanged)
       and `list` reads it, else `unknown`; the window's last-check file keeps no `curse:` version
3. [x] The source joins the router; the list import and the update check take a `curse:` entry; `Source::folders`
       from the file's `modules`; the adopt lines with an askable `curse:` source
4. [x] Fixtures (the mod, the files per type for Forever and retail, a distribution-off mod, the zip behind the
       CDN URL, a 401) and the tests of the done-when lines; the type ids as constants with a note that they
       are re-read once a key exists
5. [ ] With the key of 007: re-read the game version type ids from `GET /v1/games/1/version-types` and fix the
       constants if they differ; the manual round (`add curse:<id>` into a Forever and a retail flavor folder,
       a distribution-off project, `list`, `update --all`, `adopt`) against the real API

## Waits for the key

The code and the stubs are in; the task stays in `wip/` until the CurseForge key of task 007 arrives. Then
step 5: the game version type ids are re-read from the API (the constants of `sources::curseforge` come from
another client's reading of that endpoint), and the manual round runs against the real API and CDN, with its
dated line under Verification.

## Log

- 2026-10-22 — promoted; the code and the stubs now, the manual round when the key arrives
- 2026-10-22 — `sources::curseforge`: `CurseForge::new(http, key)`, `from_env` (the key through
  `keys::of_process`, no cache folder) and `cached` (the window's offline view: no key, nothing asked, every
  resolve an error the offline check leaves out). `project(spec)` reads `GET /v1/mods/<id>` (`CurseMod`:
  name, page `links.websiteUrl`, `allowModDistribution`, `latestFilesIndexes`; a `gameId` other than 1 is no
  addon), `files(spec, type)` reads `GET /v1/mods/<id>/files?gameVersionTypeId=<t>&pageSize=50`
  (`CurseFile`: id, `displayName`, `fileName`, `releaseType`, `fileDate`, `fileLength`, the MD5 of `hashes`,
  `downloadUrl`, `isAvailable`, `sortableGameVersions`, `modules`). The pure `game_version_type(target)`
  maps the flavor and the build onto the type ids (retail 517, Classic Era 67408, Forever 88568, TBC 73246,
  Wrath 73713, Titan 81212, Cataclysm 77522, Mists 79434; Mists without a build, a guess), kept as constants
  with a comment that they are re-read once a key exists; `file_on(files, type, channel)` takes the newest
  release on the stable channel (else the newest file), the newest file on the prerelease channel.
- 2026-10-22 — the pick: the release is the file id, the version the `displayName`, the asset the
  `fileName`, the size `fileLength`, the MD5 `hashes` of `algo` 2, the interface the newest game version of
  the type, the page the project's, the reason `CurseForge lists the <release|beta|alpha> <displayName> for
  <game>`; the Forever signal `yes` for a Forever pick, else from `latestFilesIndexes` (type 88568 named:
  `yes`, not named: `no`). `resolve_version` finds a `displayName` or a file id on the first page. The
  download checks the length and the MD5 before it is written; a 401 or 403 names `MRGL_CURSEFORGE_KEY`.
- 2026-10-22 — the key: `x-api-key` on every request to `api.curseforge.com` and `edge.forgecdn.net`
  (`keys::may_send`, whose hosts were in place since 093). `Request` gained `follow_redirects` and
  `no_redirects()`; the real client turns its redirects off for such a request (`max_redirects(0)`), and the
  source follows a redirect itself, the key going along only to a next address on the two hosts.
- 2026-10-22 — no cache: nothing of an answer is written; the answers of one run are kept in memory
  (`adopt` asks a project once for its build and its folders); `listed_versions` and the request budget are
  `None`, `cached_forever_build` is `unknown`. The sidecar entry gained the optional `forever_build`
  (`Entry::forever_build`, serde default, left out when `None`, `SCHEMA` still 1), recorded at install for a
  CurseForge pick (`install::recorded_forever_build`) and kept by a reinstall from the same source that names
  none; `sources::entry_forever_build` is the one reading of `list` and the window: the source's cache, else
  the recorded signal. The window's `last-check.json` keeps the state and the reason of a `curse:` answer and
  never its `latest` version (`last_check::kept_latest`).
- 2026-10-22 — distribution off: `Error::DistributionOff` for `allowModDistribution: false` or a file without
  `downloadUrl`, a line that names the project's page and ends with the command `mrgl add <file.zip> --as
  curse:<id>`; the update check reports
  it as `failed` with that line. `mrgl add <file.zip> --as <spec>` records the spec as the zip's source (any
  spec but a Wago page address; `--as` without a zip is refused); the version is the TOC's.
- 2026-10-22 — the router holds the source in `Sources::from_env` and `Sources::cached`; `Source::offers`
  answers from `latestFilesIndexes` and the file the channel takes (a note for a distribution-off project, a
  guess of the game type `unknown`); `Source::folders` returns the `modules` of the file this run took. The
  CLI's `forever build:` line says `CurseForge lists a file for forever` or `… no file for forever`.
- 2026-10-22 — fixtures: `tests/fixtures/sources/api.curseforge.com/v1/mods/` with `4242` (a Forever release
  and a newer beta, a retail release; the Forever file under `edge.forgecdn.net/files/7001/1/`, the retail one
  behind a 302 to `mediafilez.forgecdn.net/files/7002/2/`), `4243` (retail only), `4244` (distribution off),
  `4245` (401); the zips written by `zips/make-zips.py`, with `zips/MrglNoDistribution-2.0.0.zip` as the zip
  in hand.
- 2026-10-22 — tests: `tests/curseforge_source.rs` (the Forever and retail answers and their requests, the
  type id constants, the channels and `resolve_version`, the Forever signal and the offers of `4242` and
  `4243`, the 401, the run without a key and the offline view, the key on the two hosts only through the
  install of both flavors and never in a file under the home, the recorded signal in the sidecar file, the
  distribution-off line and its zip in hand with the update check, the update check of a `curse:` entry with
  and without the key); core unit tests of `curseforge` (the channels, the type ids, the answers, a redirect
  followed without the key off the two hosts) and of `last_check` (no version kept for a `curse:` answer);
  `tests/adopt.rs` `the_curseforge_key_makes_the_curseforge_addon_adoptable`; `tests/cli.rs`
  `add_curse_installs_from_the_stub_with_a_key_and_names_the_key_without_one`,
  `add_curse_names_the_page_of_a_project_without_distribution_and_takes_its_zip_with_as`,
  `import_takes_a_curse_entry`; the app's `flavor_list.rs`
  `a_curseforge_row_has_its_badge_and_offline_no_answer_but_its_recorded_forever_signal`.
- 2026-10-22 — docs: `docs/development.md` (the layout's `sources::curseforge` and the sidecar's
  `forever_build`, `add <file.zip> [--as <spec>]`, the keys section's redirect rule, adopt with the key, a
  Fixtures paragraph on the source and its fixtures, the `last-check.json` rule), `README.md` (the sources
  this build has, a CurseForge paragraph with the distribution-off line and `--as`, an `add curse:<id>` line
  in the usage block, adopt and import), `tests/fixtures/README.md` (the CurseForge answers and the zip in
  hand).

## Deviations from the plan

- A zip in hand with its source is `mrgl add <file.zip> --as <spec>`, for any spec but a Wago page address
  (only a request turns a slug into its id); the version is the TOC's, as for any zip in hand.
- The sidecar records the Forever signal for CurseForge picks only: the other sources keep their own caches,
  which stay fresher than the install. A reinstall from the same source that names none (a rollback, a zip
  in hand with `--as`) keeps the recorded one.
- Not in the task text: `Request::no_redirects` and the source's own redirect following, so the key never
  follows a hop of the CDN to another host (the real client keeps an `Authorization` header from a redirect,
  not a header of another name).
- The answers of one run are kept in memory, never written: the terms forbid saving or caching API data, and
  the in-memory answer of the run is what lets `adopt` ask a project once for its build and its folders.
- The window's offline view now holds a CurseForge source that asks nothing: a `curse:` row has no offline
  answer instead of `this build has no CurseForge source`, and three end-to-end tests of 092 and one window
  test that expected that line now expect the missing key, or no offline answer.
- A distribution-off project is `failed` in the update check, with its line, not a state of its own; for
  `adopt` it is a build (`yes`) with a note that it installs from a zip in hand.
- The download is checked against `fileLength` and the MD5 (`hashes` of `algo` 2); the SHA-1 is not read,
  since no SHA-1 is in the tree.
- A 403 names the key like a 401 (the CDN's answer to a request without it); the files request reads the
  first page of 50 only.

## Verification

- 2026-10-22 — stubs only, no key and no network: `cargo fmt --check`, `cargo clippy --all-targets --locked
  -- -D warnings`, `cargo test --locked` (every test green), `cargo +1.90 check --all-targets --locked` and
  `python -I scripts/changelog.py --check` pass. The done-when lines against the stubs: `add curse:4242`
  installs into `_forever_` and `_retail_` with the made-up key and names `MRGL_CURSEFORGE_KEY` without it;
  `curse:4244` names its page and installs from `MrglNoDistribution-2.0.0.zip` with `--as curse:4244`; the key
  reaches `api.curseforge.com` and `edge.forgecdn.net` only (not the redirect's `mediafilez.forgecdn.net`) and
  no file under the home folder, the sidecar included; a `curse:4242` list entry imports; `4242` classifies
  the Forever build from type 88568 and `list` shows the recorded `forever:yes` without a request. The manual
  round against the real API waits for the key (step 5).

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
      (a task session of a parallel-sessions repo: listed in the pull request under `## Follow-ups`, the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
