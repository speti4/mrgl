---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M4 Sources
---

# 097 — Adopt, the report: what source each unmanaged addon of a folder has

## Context

`mrgl adopt --flavor <id>`, read-only: per unmanaged addon the sources its TOCs name (`X-Wago-ID`,
`X-WoWI-ID`, `X-Curse-Project-ID`, `X-Website` on github.com, then the curated GitHub mapping compiled in
from `data/github-sources.json`, keyed by folder name, checked by a test, kept by pull requests), each source
asked whether it publishes a build for the flavor, and a summary: how many addons can be adopted without
CurseForge, how many only with it, how many have no known source; a `.curseclient` marker is reported;
`--json` as every command. The first half of the former 026, after 094 and 095. Decided 2026-10-22: a
CurseForge-only addon is reported here and never written by 098 before the key exists; a source without its
key reads "needs <KEY NAME>". Why: useful before any write and any key; it measures the roadmap's M4 goal.
Done when: a fixture AddOns folder with every header shape (each id header, a GitHub and a non-GitHub website,
mapping only, no hint, a multi-folder addon, an unsubstituted version) reports the sources offline against the stubs.

Decided 2026-10-22: the mapping file is compiled in with `include_str!` like `data/forever-notes.json`,
keyed by folder name, with a schema test, seeded from `lists/forever-starter.json`; fingerprint matching is
not in M4.

## Docs that move

- `docs/development.md`: the adopt module, the mapping file and its test, the TOC parser's website field
- `README.md`: the `adopt` command in the usage text
- `CONTRIBUTING.md`: how an entry joins `data/github-sources.json`, if the Forever notes have such a paragraph

## Not in this change

- Any write (098 `--apply`); the CurseForge source itself (096); fingerprint matching (not in M4)
- The window's adopt view (a follow-up once the report exists)

## Steps

1. [x] `toc.rs`: a typed `X-Website` with the github.com reader, the id shapes of the three id headers, an
       unsubstituted packager token (`@project-version@`) read as no version
2. [x] `data/github-sources.json` compiled in, keyed by folder name, a schema test, seeded from the starter list
3. [x] `adopt.rs`: the unmanaged addons of a flavor, their sources in the order of the headers then the mapping,
       each source asked for a build of the flavor (a keyless source reads "needs <KEY NAME>"), the three counts,
       the `.curseclient` marker
4. [x] `main.rs`: `mrgl adopt --flavor <id>` with `--json`, through the voice
5. [x] A fixture game folder under `tests/fixtures/adopt/` (with and without the marker) with every header shape,
       and the tests of the done-when line against the stubs of 094 and 095

## Log

- 2026-10-22 — promoted; runs on the branch of 094 and 095, 098 follows on the same branch
- 2026-10-22 — `toc`: `Toc::website` (`## X-Website`), `github_repo_of_website` (`<owner>/<repo>` of a
  github.com address, with or without `www.`, `.git` or a deeper path; `None` for any other host),
  `is_wago_id` (eight ASCII letters and digits), `is_number_id` (digits, not all zero, at most twelve) and the
  checked readers `wago_id_checked`, `wowi_id_checked`, `curse_project_id_checked`; `is_packager_token`, and
  `## Version: @project-version@` parses as no version (the raw header keeps it).
- 2026-10-22 — `sources`: `Source::offers(spec, target) -> BuildOffer` (`Offered::Yes|No|Unknown` and why)
  and `Source::folders(spec)`. The default `offers` resolves the spec (metadata `yes`, a guess `unknown`, no
  release, no asset for the flavor or a 404 `no`, another failure `unknown`), and a spec the run cannot ask is
  `unknown` without a request; WoWInterface answers from the file list's compatibility list (`yes` for the
  flavor's game, `unknown` otherwise, never `no`) and names its `UIDir` folders; Wago answers from the keyless
  slug list of the flavor's game version, with or without the key; the router hands both to the spec's
  source and answers `unknown` for a source this build lacks. `update::source_gone` moved to
  `sources::source_gone`, which both use.
- 2026-10-22 — `data/github-sources.json`: `schema`, `about` (what the file is and how an entry joins, by
  pull request, for a repository whose releases carry the addon's zip) and `folders` (folder name to
  `github:<owner>/<repo>`, one key per folder of a multi-folder addon), seeded with the 33 folders of the
  eight entries of `lists/forever-starter.json`; `adopt::MAPPING_JSON` compiles it in, `GithubMapping`
  parses it (`deny_unknown_fields`) and `problems` is the schema check.
- 2026-10-22 — `adopt::report(scan, state, sources, has_key)`: the unmanaged folders (a junction or link
  left out with a warning), their sources in the order `X-Wago-ID`, `X-WoWI-ID`, `X-Curse-Project-ID`,
  `X-Website` on github.com, the mapping, each once, from every TOC of the folder, the loaded one first; a
  website on another host is a page. Each source's `offers` (asked once per spec), what the run lacks
  (`missing`, and `needs` with the key's name when its key is not set) and whether a managed addon has it as
  its source already (`taken_by`). The folder group: the folders the source names when they include the
  folder, else the folders that name the same source, else the folder alone; the folders with a source are
  grouped first. The name is `install::derive_name` over the group, the version the named folder's TOC's.
  The row's class (`without-curseforge`, `only-curseforge`, `no-build`, `no-source`), the source it would
  be adopted as (the first usable `yes`, else the first usable `unknown`), the keys that would make it
  adoptable, the counts. `Adoptable::line`, `detail_lines`, `Report::summary`, `marker_line` and
  `left_out_lines` are the lines, in the core.
- 2026-10-22 — CLI: `mrgl adopt --flavor <flavor>` prints the addon lines, a line per source and page under
  each, the marker and the warnings, and the summary; `--json` is the `adopt` document. It reads the
  sidecar (the entries whose folders are gone dropped first, as an install does) and writes nothing.
- 2026-10-22 — fixture `tests/fixtures/adopt/`: `.build.info` with Forever and retail rows, `_forever_`
  with `MrglWagoOne` and `MrglWagoOne_Options` (one Wago id in both), `MrglNoList` (the three id headers),
  `MrglForever` (CRLF), `MrglElsewhere` with `MrglElsewhere_Options` (`UIDir` names both), `MrglCurseOnly`,
  `MrglSite` (GitHub website, `@project-version@`), `MrglNoForever` (a GitHub website without a Forever
  asset), `MrglPage` (another website), `MrglNoHint`, `Questie` with `QuestieDB` (the mapping); `_retail_`
  with a `.curseclient` marker and `MrglCompat`. Ids match the answers under `releases/` and `sources/`.
- 2026-10-22 — tests: `toc` unit tests (the id shapes, the packager token, the GitHub website reader),
  `adopt` unit tests (the compiled-in mapping against the schema and the seed, every problem the check
  names, the lookup without regard to case), `tests/adopt.rs` in the core (every header shape without a key,
  the Wago key, the marker, a managed folder and a taken source) and `crates/mrgl-cli/tests/adopt.rs` (the
  text report of every shape, the murloc line, the marker, the JSON document with the Wago key and no key
  value in the output, the missing flavor).
- 2026-10-22 — docs: `docs/development.md` (the layout's `offers`, `folders` and `adopt`, the CLI's `adopt`,
  a section "Adopt" with "The GitHub mapping", the adopt fixture under Fixtures), `README.md` (an `adopt` line
  in the usage block and a paragraph), `CONTRIBUTING.md` (a section "The GitHub mapping" next to the Forever
  notes), `tests/fixtures/README.md` (the `adopt/` folder).

## Deviations from the plan

- The folder group, a step of 098, landed with the report: the report counts addons, and the multi-folder
  addon of the done-when line is one addon, so the three grouping rules and `Source::folders` (WoWInterface's
  `UIDir`) are in `adopt::report`; 098 writes the groups.
- The summary has a fourth count when there are such addons, `with no <flavor> build at their sources`: an
  addon whose every source answers `no` is neither adoptable nor without a known source.
- Wago's answer comes from the keyless slug list of the flavor's game version (every flavor, not only
  Forever), with or without the key, so the report asks the keyed API for nothing.
- This build has no CurseForge source (096): a CurseForge source reads `needs MRGL_CURSEFORGE_KEY` while the
  key is not set, and `this build has no CurseForge source` once it is.
- A source that a managed addon has already is reported with `the source of the managed <name> already` and
  is never the source an addon would be adopted as; a junction or link mrgl did not make is left out with a
  warning.
- An `X-…-ID` of another shape is no source; its raw value stays in the typed field and in the headers, as
  `list` shows it.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 237, the core's `tests/adopt.rs` 4, the
  CLI's `tests/adopt.rs` 5, `tests/cli.rs` 147, the other targets unchanged). `cargo fmt --check`,
  `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check --all-targets --locked` and
  `python -I scripts/changelog.py --check` green.
- 2026-10-22 — by hand, the debug `mrgl` with `MRGL_HOME` under a temporary folder, the key store off, no
  discovery, `MRGL_HTTP_STUB` at a temporary copy of `tests/fixtures/releases/` and `tests/fixtures/sources/`
  and `MRGL_ROOT` at `tests/fixtures/adopt/`: `adopt --flavor forever` reported the ten addons as the tests
  expect (six adoptable without CurseForge, one only with CurseForge, two with no known source, one with no
  Forever build), `adopt --flavor retail` named the `.curseclient` marker; no sidecar was written. No game
  folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
