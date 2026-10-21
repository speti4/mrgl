---
size: M
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
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

1. [ ] `sources.rs`: the `Source` trait, the pick type, the network trait with a local stub
2. [ ] `sources/github.rs`: releases API, `release.json` parsing, the flavor pick and the name heuristics,
       the `ETag` cache under `MRGL_HOME`
3. [ ] Fixture `release.json` files and tests for every shape, offline
4. [ ] `mrgl-cli`: `add github:<owner>/<repo> --flavor <id> [--dry-run]`, the voice lines, `--json`,
       end-to-end tests with the stub
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, when the work is done)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups` (the main session files them)
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the stale index is the expected finding on a task branch)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`, `CHANGELOG.md` regenerated
