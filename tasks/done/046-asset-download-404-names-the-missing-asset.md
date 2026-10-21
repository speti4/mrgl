---
size: S
origin: done/040-add-github-installs-the-pick-through-the-engine.md
created: 2026-10-21
started: 2026-10-22
closed: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 046 — The asset download's error path: a 404 names the asset, the size is checked

## Context

When the GitHub source's asset download answers 404, the error names the asset and the release
instead of the release-list hint ("no such repository, or it is private"); and the downloaded asset's byte
count is compared with the `size` the release list names for it, a mismatch refused before the engine
opens the zip, with a line that names both numbers (a release without a size installs as before). The
hint sends the user to check the repository name when the asset was renamed, and a truncated download
today surfaces only as an unreadable zip, which reads like a packaging bug.
Done when: a fixture release whose asset is missing from the stub fails `add` with a line naming the
asset, a fixture asset shorter than its listed size is refused with the documented line, and the
release-list 404 keeps its hint.

Decided 2026-10-22: second of the chain 042 → 046 → 041.

## Docs that move

- `docs/development.md` (the source's errors) if the error list there changes; otherwise `none`.

## Not in this change

- The offline resolve (041, next in the chain); a retry of a failed download; a checksum from the release
  (GitHub publishes none).

## Steps

1. [x] The download's 404 is its own error naming the asset and the release tag; the release-list 404
       keeps its hint
2. [x] The byte count of the downloaded asset against the release list's `size`; a mismatch refused with
       both numbers before the engine opens the zip; no `size` installs as before
3. [x] Tests on the stub: a missing asset, a short asset, a release without a size, the release-list 404
4. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 042 → 046 → 041
- 2026-10-22 — `error.rs`: two variants at the end of the enum. `AssetMissing` reads `the asset <name> of
  <source> <tag> is missing: GitHub answered 404 for <url> (removed or renamed after the release was
  listed)`; `AssetSizeMismatch` reads `refused <asset> of <source> <tag>: <n> bytes downloaded, the release
  list names <size>; the download was cut short or the asset changed, try again`.
- 2026-10-22 — `sources/github.rs`: the asset download (`download_asset`, for the picked zip and for
  `release.json`) maps a 404 to `AssetMissing` with the source, the tag and the asset name; every other
  status stays the status error with its hint, and the release list's 404 keeps `no such repository, or it
  is private`. `Source::fetch` compares the body's byte count with `Pick::size` before writing the file,
  so `Engine::install_pick` never reaches the install call with a cut download; a pick without a size is
  written as it came.
- 2026-10-22 — fixture: the Forever asset of `Questie/Questie` v1.2.3 no longer lists a `size` (the zip
  behind it is 486 bytes, the list said 2048, and the import tests of task 015 put other fixture zips
  behind it); it is the release-without-a-size case of every install test that uses it.

## Deviations from the plan

- The 404 rule covers every asset download, `release.json` included, not only the picked zip.
- The size check sits in `GitHub::fetch` rather than in the engine: the source owns the release list's
  `size`, and refusing before the file is written leaves nothing in the staging folder.
- `docs/development.md` had no error list; the two lines are described in a new paragraph next to the
  source's cache.

## Verification

- 2026-10-22 — `cargo test --locked`: every test green. `tests/github_source.rs`
  `a_missing_asset_names_the_asset_and_the_release` fetches the `github:Example/NoReleaseJson` Forever pick,
  whose zip the stub does not serve, and gets `the asset Thing-3.0.0-forever.zip of
  github:Example/NoReleaseJson 3.0.0 is missing: GitHub answered 404 for
  https://github.com/Example/NoReleaseJson/releases/download/3.0.0/Thing-3.0.0-forever.zip (removed or
  renamed after the release was listed)` with no file written, while `github:Example/Missing` still ends in
  `: no such repository, or it is private`; `the_downloaded_size_is_checked_against_the_listed_one` serves a
  9-byte asset listed at 2048 (`refused A-forever.zip of github:o/r v1: 9 bytes downloaded, the release list
  names 2048; the download was cut short or the asset changed, try again`, nothing written) and writes it
  when the list says 9, says nothing or says `null`. The unit test
  `a_missing_release_json_download_names_the_asset` covers the `release.json` 404 and keeps a 503 the
  status error. `tests/cli.rs` `add_refuses_an_asset_shorter_than_its_listed_size` runs `mrgl add
  github:Example/Short --flavor forever` against a stub with a 12-byte asset listed at 2048: exit 1, the
  refusal as the last line (and as the `--json` error document's message), a `downloading` line and no
  `installing` line, an empty staging folder, no sidecar and no addon folder;
  `add_github_without_dry_run_needs_the_game_folder_and_the_asset` now expects the missing-asset line. The
  040 install tests over `Questie/Questie`, whose Forever asset lists no size, install as before.
- 2026-10-22 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo +1.90 check
  --all-targets --locked` and `python -I scripts/changelog.py --check` green.

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the README index is the main session's)
- [x] moved to `done/` with `closed: YYYY-MM-DD`
