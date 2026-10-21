---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
closed: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 003 — GitHub release source: `release.json`, flavor-correct asset pick, ETag cache; CLI `add github:<owner>/<repo>`

## Context

The first source of D-002: fetch a repository's releases over the GitHub REST API (an optional token from
`MRGL_GITHUB_TOKEN`), read the release's `release.json` as the BigWigs packager writes it, pick the asset
whose metadata names the flavor (flavor key and interface number), fall back to asset-name heuristics
(`-forever`, `-classic`, `-mainline`) flagged as a guess, and cache the release list with `ETag` under the
mrgl home folder. `sources.rs` gets a `Source` trait that resolves a repository and a flavor to a release
pick (asset name, download URL, version, interface, whether it is a guess) and fetches the asset to a path;
the network sits behind a trait with a local stub, so every test runs on fixture `release.json` files
(several flavors, no metadata, no `release.json`). `mrgl add github:<owner>/<repo> --flavor <id>
[--dry-run]` resolves and reports the pick; without `--dry-run` it reports the pick and names task 004 for
the install until the engine lands.
Why: the Forever addon list publishes on GitHub with flavor metadata; this source covers the Forever list.
Done when: `add github:Questie/Questie --flavor forever --dry-run` names the `forever` asset, the fixture
`release.json` files resolve as documented, and no test touches the network.

Decided 2026-10-21: the HTTP client is a small crate with rustls (no OpenSSL, nothing to install, D-002),
its license on the allowlist; the resolved pick is the handover to the install engine of 004.

## Docs that move

- `crates/mrgl-core/Cargo.toml`: the HTTP crate joins with its one-line reason; `deny.toml` if a license
  needs the allowlist.
- `docs/development.md` ("Layout", "Fixtures"): the `sources` module, the `release.json` fixtures and the
  `MRGL_GITHUB_TOKEN` variable.
- `README.md`: `mrgl add github:<owner>/<repo> --flavor forever` as a usage line.
- `crates/mrgl-core/tests/fixtures/README.md`: the `releases/` fixtures.

## Not in this change

- The install itself (004) and `update --all` (006).
- The release body line before an update (030).
- Wago, WoWInterface and CurseForge (M4).

## Steps

1. [x] `sources.rs`: the `Source` trait, the pick type, the network trait with a local stub
2. [x] `sources/github.rs`: releases API, `release.json` parsing, the flavor pick and the name heuristics,
       the `ETag` cache under `MRGL_HOME`
3. [x] Fixture `release.json` files and tests for every shape, offline
4. [x] `mrgl-cli`: `add github:<owner>/<repo> --flavor <id> [--dry-run]`, the voice lines, `--json`,
       end-to-end tests with the stub
5. [x] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)
- 2026-10-21 — `ureq` 3.4 (MIT OR Apache-2.0) with `default-features = false, features = ["rustls"]` is the
  HTTP client; its rustls stack brings `ring`, `rustls-webpki` and `untrusted` (ISC) and `webpki-roots`
  (CDLA-Permissive-2.0), so both licenses join `deny.toml` and `about.toml`; `cargo deny check` is green.
- 2026-10-21 — `sources.rs`: `SourceSpec` (`github:<owner>/<repo>`, also the repository URL), `Target`
  (flavor id and the installed build), `Pick` (source, tag, version, published, prerelease, asset, URL,
  size, flavor key, interface, `guess`, reason), the `Source` and `Http` traits, `UreqHttp` (mrgl's user
  agent, connect and response timeouts, statuses returned rather than raised, no `Authorization` across a
  redirect) and `FolderHttp`, the stub that answers from a folder laid out as URL paths, with an `ETag` from
  the content and a 304 on a matching `If-None-Match`; `MRGL_HTTP_STUB` selects it in the exe.
- 2026-10-21 — `sources/github.rs`: the releases API with `Accept`, `X-GitHub-Api-Version` and the optional
  bearer token (API only, never on a download); the current release is the newest non-draft stable one, a
  prerelease only when nothing else exists; the flavor keys per target (`forever`/`camelot`,
  `mainline`/`standard`, `classic`/`vanilla`, and for the progressing classic the game type of the installed
  build's major version, every game type newest first when the build is unknown, flagged as a guess); the
  `release.json` pick prefers the library zip over `nolib` and the interface of the build's major; the
  name heuristics match `-<key>` before `.zip` and an optional `-nolib`, the retail zip being the one with
  no flavor suffix, always a guess; the statuses carry hints (404, 401, the rate limit with the token
  variable, 5xx); the release list is cached as one file with its tag.
- 2026-10-21 — fixtures under `tests/fixtures/releases/`: `Questie/Questie` (a draft above a stable release
  naming four flavors with `nolib` twins), `Example/NoMetadata`, `Example/NoReleaseJson`, `Example/Empty`,
  `Example/Prerelease`, `Example/BetaOnly`, `Example/MainlineOnly`; 9 integration tests over them, 13 unit
  tests with an in-memory stub, 4 unit tests of `sources.rs`.
- 2026-10-21 — the command line: `add <source> --flavor <flavor> [--dry-run]` resolves the target (a flavor
  folder path, or a name or id with the installed build read from the game folder when `MRGL_ROOT` or the
  remembered root holds it), says `resolving …` (progress), `pick for <flavor>: …` (success) or `pick for
  <flavor>, a guess: …` (warning), then `dry run: nothing installed` (nothing) or fails with `add (the
  install) is not implemented yet (task 004)`; `--json` is one `add` document with the pick; 8 end-to-end
  tests with `MRGL_HTTP_STUB` set on every run.
- 2026-10-21 — a concurrent cache write made one end-to-end test flaky (two runs sharing one home folder):
  the cache became one file, written through a temporary file and a rename, and a failed cache write never
  fails the command.

## Deviations from the plan

- The cache is one file, `<home>/cache/github/<owner>/<repo>/releases.json`, holding the tag and the list
  together, instead of a body file and a tag file: two files can disagree between two runs, one cannot.
- An unreadable `release.json` does not fail the pick: the source falls back to the asset names and the
  pick's reason starts with the parse error, flagged as a guess.
- The fixtures live under `tests/fixtures/releases/` laid out as URL paths (the planned `release-json/`
  folder), because the same folder serves the exe through `MRGL_HTTP_STUB`, which exists in the shipped
  command line as a documented test hook.
- The flavor keys accept the client's TOC-suffix spellings as synonyms (`camelot`, `standard`, `vanilla`,
  `wotlkc`, `tbc`) next to the packager's keys.

## Verification

- 2026-10-21 — `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test
  --locked` (31 CLI end-to-end tests, 48 + 9 + 10 core tests), `cargo +1.90 check --all-targets --locked`,
  `python -I scripts/changelog.py --check` and `cargo deny check` green in the worktree; the CLI suite run
  three times in a row after the cache fix.
- 2026-10-21 — `mrgl --serious add github:Questie/Questie --flavor forever --dry-run` with `MRGL_HTTP_STUB`
  on the fixtures prints `resolving github:Questie/Questie for forever`, `pick for forever:
  github:Questie/Questie v1.2.3 (Questie-v1.2.3-forever.zip, flavor forever, interface 16001): release.json
  names forever (interface 16001)` and `dry run: nothing installed`, exit 0; without `--dry-run` the same
  pick, then `add (the install) is not implemented yet (task 004)`, exit 1.
- 2026-10-21 — the fixture shapes resolve as the fixtures README documents: every flavor of
  `Questie/Questie` from `release.json` (the library zip, Mists from the build 5.5.4, a guess without a
  build), `NoMetadata` and `NoReleaseJson` by name as guesses, `Empty` as `no release`, `Prerelease` to the
  stable tag, `BetaOnly` to the prerelease, `MainlineOnly` with the flavors it names in the error.
- 2026-10-21 — no test touches the network: every end-to-end run sets `MRGL_HTTP_STUB`, the unit and
  integration tests build `FolderHttp` or the in-memory stub, and `UreqHttp` is constructed only in
  `http_from_env` when the variable is unset. No manual round on a real repository in this change (the
  first one is a follow-up).

## Closing checklist

- [x] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green (the stale index is the expected finding on a task branch)
- [x] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
