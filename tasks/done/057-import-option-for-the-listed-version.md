---
size: S
origin: done/015-list-export-import-and-interface-warning.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 057 — An import option that installs the listed version, not the current release

## Context

`import --exact` (or the name the CLI's flags prefer) installs the version the list file records
for each entry, through the source's release list (the tag of that version), and reports an entry whose
version the source no longer offers; the default stays the current release. A list exported from a
working folder is a known-good set; a second machine wants that set, not whatever shipped since.
Done when: a fixture list with an older version installs that version from the stub, and a version the
stub lacks is reported, not installed.

Decided 2026-10-22: third in the chain 055 → 006 → 057; the exact install goes through the one install
helper of 055 with a resolve of a named version instead of the latest pick.

## Docs that move

- `README.md` ("What it will do") and `docs/development.md`: the import flag and what a missing version
  reports.

## Not in this change

- Entries of other sources (056); a list file that names a local path (028); the shareable list format
  itself (028).

## Steps

1. [x] The source resolves a named version (the release tagged with it) next to the latest pick
2. [x] `import --exact` through the install helper of 055; a version the source lacks is reported, the
       entry skipped, the summary counts it
3. [x] Tests: a fixture list with an older version installs it from the stub; a version the stub lacks is
       reported; the default import unchanged
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), third
  of the chain 055 → 006 → 057
- 2026-10-22 — `sources.rs`: `Source::resolve_version(spec, target, version)` next to `resolve`, with a
  default for a source that knows only its current release (that release when it carries the version,
  else `Error::VersionNotOffered`, `no release <version> in <spec>`, the new last variant of the error
  enum). `sources/github.rs`: `release_of_version(releases, version)` is the release whose tag is the
  version, with or without its leading `v`, never a draft, on any channel; `GitHub::resolve_version` takes
  it from the same cached release list, and the asset choice of `resolve` moved into `GitHub::pick_of`,
  which both call.
- 2026-10-22 — `install.rs`: `Wanted::Version(String)` resolves through `Source::resolve_version`, so the
  exact install is the one install path of 055 with another resolve. `list_file::import(engine, source,
  list, exact, dry_run)` asks for `Wanted::Version(entry.version)` with `exact`; `VersionNotOffered` becomes
  the outcome `version-not-offered` (skipped, counted), every other error stays `failed`.
- 2026-10-22 — `mrgl-cli`: `import --exact`; the progress line adds `at the versions it names`, the data
  line of a missing version is `<name>  skipped, version <v> not offered: no release <v> in <source>`, the
  summary counts `version not offered` (a warning, exit 0), the JSON document carries `exact`.
- 2026-10-22 — tests: `tests/cli.rs` `import_exact_installs_the_listed_version_and_reports_one_the_source_lacks`
  and `import_without_exact_still_takes_the_current_release` over a copy of the fixture answers with the
  fixture `Questie` zip behind the v1.2.2 Forever asset; `github::tests` for the lookup and the resolve of
  a version.
- 2026-10-22 — on top of tasks 042, 046 and 041: `GitHub::resolve_version` reads the same paged and
  cached list as `resolve` (`release_list`), so an exact pick offline comes from the cache with its stale
  flag, and the import lists `Pick::stale_warning()` among the entry's warnings, the dry run's too.

## Deviations from the plan

- The option is `--exact`, the name the task proposed; it combines with `--dry-run`.
- A version the source no longer offers is its own outcome (`version-not-offered`), a warning that does not
  fail the run, rather than a failure: the list is still a known-good set for the other entries.
- The lookup sees the releases the release list holds: the first page of 30, and the pages after it only
  while none holds a stable release (task 042); an older version beyond them reads as not offered.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green (core unit 145, `tests/cli.rs` 88 with 1 ignored).
  `import exact.json --flavor forever --exact --dry-run` against the stub copy prints `dry run: reading 2
  addons from exact.json into _forever_ at the versions it names`, `Questie  would install 1.2.2 from
  github:Questie/Questie`, `Notes  skipped, version 0.5.0 not offered: no release 0.5.0 in
  github:Example/Notes` and `dry run: 2 addons of exact.json for _forever_: 1 to install, 1 version not
  offered; nothing written`, with no sidecar written. Without `--dry-run`: `Questie  installed 1.2.2 from
  github:Questie/Questie`, `imported 2 addons of exact.json into _forever_: 1 installed, 1 version not
  offered`, exit 0; the sidecar records `github:Questie/Questie` at 1.2.2, `Questie\Questie.toc` is there,
  and nothing of `Notes` is. The `--json` run has `exact: true`, `installed` at 1.2.2 and
  `version-not-offered` with `listed_version` 0.5.0. The same list without `--exact` installs 1.2.3 (`Questie
  installed 1.2.3 from github:Questie/Questie, the list had 1.2.2`). `github::tests`: `v2.0.0-beta` is found
  as `2.0.0-beta` and as `v2.0.0-beta`, a plain `1.0.0` tag by itself, a draft's version and an unknown one
  are not; `resolve_version` over a two-release list picks `A-v1.0.0-forever.zip` of `v1.0.0` while
  `resolve` takes 1.1.0, and `0.9.0` is `no release 0.9.0 in github:o/r`.
- 2026-10-22 — `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
