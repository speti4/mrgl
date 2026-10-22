---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M4 Sources
---

# 094 — The WoWInterface source

## Context

`wowi:<id>` resolves through the MMOUI API (`filedetails/<id>.json`, no key), one file per project; the
flavor comes from the project's compatibility list (a metadata pick for retail and the classic flavors, a
guess without one and for Forever), a pending file is reported and not installed, the download is checked
against the API's MD5, the answer is cached under `cache\wowi\` (the API allows an hour), and the Forever
signal stays `unknown` until WoWInterface lists a Forever compatibility. Runs after 092 and 093, before 095.
Decided 2026-10-22: the first new source, since it works without a key; the interface check after install
is the safety net of a Forever guess. Why: the one keyless source after GitHub, and the proof of the router.
Done when: `add wowi:<id>` installs from the stub, a compatibility list picks by metadata and none is a
guess, a pending file is reported, a wrong MD5 is refused before the zip opens, a list entry imports, and a
fixture answer classifies the Forever build as `unknown`.

Decided 2026-10-22: the Forever signal is `unknown` wherever the API does not name Forever, never inferred
from a classic or retail compatibility; the WoWInterface answers are cached like GitHub's.

## Docs that move

- `docs/development.md`: the sources layout (the WoWInterface module, its cache folder, its fixtures)
- `README.md`: the source list and the `wowi:<id>` spec in the usage text
- `Cargo.toml`: the one-line reason if an MD5 crate is added (else a hand-rolled MD5 with test vectors)

## Not in this change

- The Wago source (095) and the CurseForge source (096); the adopt report (097)
- A Forever signal inferred from a non-Forever compatibility

## Steps

1. [x] `sources/wowinterface.rs`: resolve by id through `filedetails/<id>.json`, the flavor from the
       compatibility list (metadata pick for retail and the classic flavors, a guess for Forever and without
       a list), a pending file reported and not installed, the channels, the cache under `cache\wowi\`
2. [x] The download checked against the API's MD5 before the engine opens the zip; the Forever signal `unknown`
3. [x] The source joins the router of 092 (`Sources::from_env`, `Sources::cached`); the list import and the
       update check take a `wowi:` entry
4. [x] Fixtures under `tests/fixtures/sources/` (an answer with a compatibility list, one without, a pending
       file, a wrong MD5, the zip behind the download URL with the stub's query mapping) and the tests of the
       done-when lines

## Log

- 2026-10-22 — promoted; runs on the branch of 092 and 093, 095 follows on the same branch
- 2026-10-22 — the compatibility list: `UICompatibility` is only in the all-projects `filelist.json` (some
  5.6 MB for 8226 projects; `filedetails` lacks it, and the API sends no `ETag`). Decided: the source reads
  that list at most once a run and once a day, and keeps it trimmed to each project's compatibility, folders
  (`UIDir`) and page under `<home>/cache/wowi/filelist.json` (about 1.5 MB for the live list); the cache of
  any age stands in when the API cannot answer, and a list that cannot be read at all makes the pick a guess
  that says so. The folders are kept for the adopt work of 097 and 098, which needs a project's folder group.
- 2026-10-22 — MD5: decided for RustCrypto's `md-5` 0.10 over a hand-rolled one. It is the family `sha2`
  comes from, so its `digest` and buffer crates were in the tree already: `Cargo.lock` gains the one
  package; MIT or Apache-2.0, `cargo deny check` green. The RFC 1321 test suite runs against it in the core's
  unit tests all the same.
- 2026-10-22 — `sources::wowinterface`: `WowInterface::new(http, cache_dir)` and `from_env`;
  `file_details(id)` (the API's array of one, `FileDetails`; offline the cached answer with `Stale`; a 404 is
  `WoWInterface answered 404 for <url>: no such project, or it was removed`), `file_list()`, `listing(id)`
  and `cached_listing(id)` (`Listing`: `compatibility`, `dirs`, `page`); the pure `game_of(version)` (`1.13`
  to `1.59` Classic Era, `1.60` and up Forever, `2` to `5` the progressing classic, Titan's `3.80` apart from
  Wrath's `3.4`, `6` and up retail), `compatibility_for(list, target)` (the newest entry of the target's
  game, and whether the game type is a guess) and `forever_build_of(listing)`. The resolve: a pending file
  (`UIPending: "1"`) is the new `Error::ReleasePending`; the pick's release is `UIVersion`, its version that
  text without a leading `v` (the tag rule, now `sources::version_of_tag`, which `github::Release::version`
  uses too), `published` from `UIDate`, `notes` from the changelog's first line, `page` from the list, `md5`
  from `UIMD5`, `interface` none (the list names game versions, not the TOC's interface); a metadata pick
  says `WoWInterface lists compatibility <version> (<patch name>)`, a guess says what the list names and
  that the project's one file is taken. Both channels take the one file.
- 2026-10-22 — the MD5 check: `Pick` gains `md5` (left out of the JSON when `None`); `sources::check_md5`
  compares the download before `fetch` writes it, a mismatch is the new `Error::AssetChecksumMismatch`. The
  Pick literals of the tests and of the router's test source gained `md5: None`.
- 2026-10-22 — the router: `Sources::from_env` and `Sources::cached` hold the WoWInterface source (the
  window's offline view reads its cached answers and list). The CLI's `forever build:` line says why in the
  source's own words (`sources::forever_why`): `WoWInterface lists a Forever compatibility` or `lists no
  Forever compatibility`; the GitHub lines are unchanged.
- 2026-10-22 — fixtures: `tests/fixtures/sources/` with `api.mmoui.com/v3/game/WOW/filedetails/<id>.json`
  for `1001` (retail, Classic Era, Cataclysm Classic), `1002` (no compatibility), `1003` (pending), `1004`
  (a wrong MD5), `1005` (a made-up Forever version), a trimmed `filelist.json` (with `1006`, two folders,
  in the list alone) and the stored zips behind the download addresses (`getfile.php@d=1785000000&id=<id>&minion`),
  written by `zips/make-zips.py`, which now writes these uncompressed so the MD5 never depends on the zlib.
- 2026-10-22 — tests: `tests/wowinterface_source.rs` (the metadata picks per flavor, the guesses without a
  list and on Forever, the Forever `yes` of `1005` and `unknown` of `1001`, the pending file, the MD5 check
  before the write, the 404, the file list read once a run and once a day, the offline answer and the Forever
  signal from the cache alone, another source's spec); core unit tests of `wowinterface` (the games of the
  versions, the entry per flavor, the signal, the reasons, the details' fields) and of `sources` (the RFC
  1321 vectors, `check_md5`, `version_of_tag`, `forever_why`); `tests/cli.rs`
  `add_wowi_installs_the_projects_one_file_from_the_stub`,
  `add_wowi_reports_a_pending_file_and_refuses_a_wrong_md5_before_the_zip_opens`,
  `import_and_update_take_a_wowi_entry`.
- 2026-10-22 — docs: `docs/development.md` (the layout's `sources::wowinterface`, a Fixtures paragraph on
  the source, its cache and its fixture projects), `README.md` (the sources this build has, how a
  WoWInterface pick is made, the Forever line, the import, an `add wowi:<id>` line in the usage block),
  `tests/fixtures/README.md` (`sources/`), `Cargo.toml` (the reason line of `md-5`).

## Deviations from the plan

- Three end-to-end tests of 092 used `wowi:123` as the spec of a source this build lacks
  (`add_of_a_source_this_build_lacks_names_what_is_missing`,
  `import_reads_a_list_entry_of_a_source_this_build_lacks_as_not_configured`,
  `update_does_not_ask_a_source_this_build_lacks_and_says_what_is_missing`); they now use a CurseForge spec
  (`curse:77` from its page address, `curse:123`), the source this build still lacks, and assert the same.
- A pending file in the update check is a failed check (`check failed: the newest file of wowi:<id> waits
  for approval on WoWInterface: …`), which makes `mrgl update` end with exit code 1 like any other failed
  check; no state of its own was added, since a new state reaches the window's state list too.
- The pick names no `interface` for a WoWInterface file: the compatibility list names game versions the
  author ticked, not the TOC's interface, so the interface check after the install stays the judge.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 228, `tests/wowinterface_source.rs` 9,
  `tests/cli.rs` 144, the other targets unchanged). `cargo fmt --check`, `cargo clippy --all-targets
  --locked -- -D warnings`, `cargo +1.90 check --all-targets --locked`, `cargo deny check` (advisories,
  bans, licenses, sources ok) and `python -I scripts/changelog.py --check` green; `make-zips.py` rewrites
  every earlier zip byte for byte.
- 2026-10-22 — by hand, the debug `mrgl` with `MRGL_HOME` under a temporary folder and `MRGL_HTTP_STUB` at
  `tests/fixtures/sources/`: `add wowi:1001 --flavor retail --dry-run` picks by metadata (`WoWInterface lists
  compatibility 12.0.7 (Made-up Patch)`), `--flavor era` by `1.15.7`, `add wowi:1002` is a guess, `add
  wowi:1003` fails with the pending line (exit 1); the cache folder held `1001.json`, `1002.json`,
  `1003.json` and `filelist.json`. One read-only dry run against the public API without the stub: a real
  project resolved for retail by metadata and for Forever as a guess with `forever build: unknown`, and the
  trimmed list cache was about 1.5 MB. No game folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
