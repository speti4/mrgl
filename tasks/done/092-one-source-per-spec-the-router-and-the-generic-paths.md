---
size: M
origin: session 2026-10-22
created: 2026-10-22
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M4 Sources
---

# 092 — One source per spec: the router, the spec forms and the generic paths

## Context

`SourceSpec` gains `wago:<id>`, `wowi:<id>` and `curse:<id>` (and the addon page URLs an id can be read
from), a `Sources` value implements `Source` by dispatching on the spec and replaces every direct `GitHub`
construction in the command line and the window; the Forever build signal moves onto the trait (default
`unknown`); the list import and the update check ask any configured source, and an entry of a source this
build lacks reads "source not configured", not "not importable"; the test stub maps a query string to a
file and `Http` gains `post`. The first task of M4: the source tasks 094 to 096 plug in here.
Why: every caller is GitHub-only today, so each new source would touch the same eight places.
Done when: every existing test passes unchanged, `add wago:x` without the Wago source names what is missing,
a `wowi:` list entry reads "source not configured", and the stub serves two flavors of one URL from two files.

Decided 2026-10-22: the first task of M4, before the key store (093) and the sources (094 to 096); the
Forever signal is `unknown` wherever a source does not name Forever, never inferred.

## Docs that move

- `docs/development.md`: the sources layout (the router, the spec forms), the fixture stub's query mapping
- `README.md`: the source spec forms the commands accept, if the usage text lists them

## Not in this change

- Any real second source (094 WoWInterface, 095 Wago, 096 CurseForge) and the key store (093)
- The adopt report and its TOC header reading (097, 098)

## Steps

1. [x] `sources.rs`: the three new `SourceSpec` variants, their parse forms (spec and page URL) and canonical
       display; the Forever signal on the `Source` trait with the `unknown` default
2. [x] A `Sources` router value built once from the environment and used by every CLI and window call site
       (`main.rs`, `commands.rs`, `list.rs`, the offline view of 078 per source)
3. [x] `list_file.rs` and `update.rs`: a configured source is asked; a source this build lacks reads
       "source not configured"; `install.rs` name hints per spec
4. [x] The folder HTTP stub maps a query string to a file; `Http` gains `post`, stubbed the same way
5. [x] Tests: the done-when lines; every existing test passes unchanged

## Log

- 2026-10-22 — promoted; runs on its own branch, 093 follows on the same branch
- 2026-10-22 — spec forms: `SourceSpec::{Wago, WowInterface, CurseForge}` with `id`, read from `wago:<id>`
  (ASCII letters and digits as written, at most 32), `wowi:<id>` and `curse:<id>` (a number above zero,
  leading zeros dropped), and from the page addresses that carry an id: `github.com/<owner>/<repo>` (now
  also with `www.` and a query), `www.wowinterface.com/downloads/info<id>[-<name>][.html]` and
  `www.curseforge.com/projects/<id>`. A Wago page (`addons.wago.io/addons/<slug>`) and a CurseForge addon
  page (`curseforge.com/wow/addons/<slug>`) name a slug, which is not the id: they are refused with a hint
  that names the spec form and where the id is (the TOC's `X-Wago-ID` / `X-Curse-Project-ID`, the project
  id on the CurseForge page). Display is canonical (`wago:<id>`, `wowi:<id>`, `curse:<id>`). A
  `SourceKind` names the four sources (prefix, display name, parse) for the router and the key store of 093;
  `SourceSpec::kind` and `SourceSpec::name_hint` (the GitHub repository; an id is no name) join it.
- 2026-10-22 — the trait: `Source::missing(spec)` (what the run lacks to ask, default nothing),
  `Source::cached_forever_build(spec, channel)` (default `unknown`; the GitHub source's former inherent
  method is now its implementation of it) and `Source::request_budget_for(spec)` (default
  `request_budget()`); `sources::not_configured` turns a `missing` clause into
  `Error::SourceNotConfigured`, `source not configured: <spec> (<what is missing>)`. Handed a spec of
  another source, the GitHub source answers that error instead of the five irrefutable matches it had.
- 2026-10-22 — the router: `sources::Sources` (`sources/router.rs`), one slot per `SourceKind`,
  implements `Source` by dispatching on the spec (the download on the pick's `source`); an empty slot is
  `this build has no <Name> source`. `Sources::from_env()` builds the sources of the process over one
  `Arc<dyn Http>` (`Http` is implemented for `Arc`); `Sources::cached(http, home)` builds them over a
  caller's client with the caches under a home folder and no key, which is the window's offline view
  (task 078) over its `Offline` client. Every former `GitHub::from_env` call site uses the router: CLI
  `list` (Forever signal), `add`, `import`, `update`; the window's `resolve`, the update actions and the
  flavor list's offline check and rows.
- 2026-10-22 — the generic paths: `EntrySource::GitHub` became `EntrySource::Spec` for any of the four
  specs, `Unsupported` is left for a spec mrgl does not read; the import reports an entry whose source the
  run cannot ask as the new outcome `SourceNotConfigured` (`source-not-configured`, detail: what is
  missing, before any request; the CLI line `source not configured (<source>): <what is missing>`, counted
  in the summary, which is then a warning); the update check asks an entry only when its source is
  configured, and leaves one that is not `not checked: source not configured: <spec> (<what is
  missing>)`, and reads the budget of the spec at hand. `add` checks the spec against the router right
  after parsing, before the game folder or the network is read. `install::name_hints` takes the spec's
  name hint. The window's badge of a managed `wago:`, `wowi:` or `curse:` entry is `wago`, `wowi`,
  `curseforge`, the kinds its TOC hints already used.
- 2026-10-22 — the stub: `Request` carries an optional body (`Request::post`), `Http::post` sends it
  (`UreqHttp` through ureq's `post().send()`; a default that refuses as the network does, so a GET-only
  stand-in stays a few lines); `FolderHttp` answers a URL from `<path>@<query>` first (the pairs sorted by
  name, `page` left out, a character Windows refuses in a file name written `_`), with or without
  `.json`, then from the path alone as before, so every existing fixture answers unchanged; a POST is
  answered from the same files and kept with its body.
- 2026-10-22 — tests: core unit `sources` (the spec forms and their canonical display, the bad specs and
  their hints, `SourceKind`, two queries of one path from two files, a POST and its body), `router` (the
  dispatch, a source this build lacks and a source that lacks a key, the cached sources offline),
  `list_file` (a `wowi:` entry reads `source not configured`, an unknown prefix stays not importable),
  `install` (name hints of the new specs); `tests/cli.rs`
  `add_of_a_source_this_build_lacks_names_what_is_missing`,
  `import_reads_a_list_entry_of_a_source_this_build_lacks_as_not_configured`,
  `update_does_not_ask_a_source_this_build_lacks_and_says_what_is_missing`; the window's
  `tests/flavor_list.rs` `a_row_of_a_source_this_build_lacks_has_its_badge_and_says_what_is_missing`.
- 2026-10-22 — docs: `docs/development.md` (the layout's sources: the spec forms, the router, the trait's
  new methods; the import and the window's offline view through the router; the stub's query mapping and
  POST under Fixtures), `README.md` (the spec forms `add` accepts and what a source not configured does).

## Deviations from the plan

- Four existing tests pinned the behavior this task changes and were updated, not kept unchanged: the
  core unit `a_bad_spec_names_the_accepted_shape` and `the_source_kind_follows_the_spec` listed `wago:` as
  an unknown spec and an unsupported entry, and `tests/cli.rs` `an_unknown_source_spec_is_a_plain_error`
  and `add_failures_are_plain_lines_with_exit_code_one` expected `unknown source: wago:… (expected
  github:<owner>/<repo>)`; the first two now cover the new forms and hints, the CLI ones use an unknown
  prefix (`bitbucket:`) and the `source not configured` line. `tests/github_source.rs` had an irrefutable
  `let SourceSpec::GitHub { .. } = &spec;`, which a second variant makes refutable: it gained an
  `else { unreachable!() }`, nothing else. Every other test passes unchanged.
- `EntrySource::Unsupported` did not split into two variants: `EntrySource::Spec` holds any spec, and
  whether the run can ask it is the sources' answer (`Source::missing`), since a configured source
  depends on the run (a key) and not on the spec.
- The update check's stop before a rate limit stays one stop for the whole check: it reads the budget of
  the spec at hand, but once one source's budget is used up the entries of every source after it are not
  asked. Only GitHub has a budget in this build.
- `http_from_env` stays public; the router is built on it.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (`tests/cli.rs` 138, core unit 209,
  `tests/flavor_list.rs` 17, `tests/github_source.rs` 32, `tests/install.rs` 42 and the other targets
  unchanged). `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90
  check --all-targets --locked` and `python -I scripts/changelog.py --check` green.
- 2026-10-22 — the debug `mrgl` with `MRGL_HOME` under a temporary folder and `MRGL_HTTP_STUB` at the
  fixture answers: `add wago:L6Jb2R6v --flavor forever` prints the murloc line and `source not configured:
  wago:L6Jb2R6v (this build has no Wago source)`, exit 1; `add
  https://www.wowinterface.com/downloads/info5108-Clique.html --dry-run` reads `wowi:5108` and fails the
  same way; a CurseForge addon page is `unknown source` with the slug hint; `add
  https://github.com/Questie/Questie --flavor forever --dry-run` picks `Questie-v1.2.3-forever.zip` as
  before. No real game folder was read.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
