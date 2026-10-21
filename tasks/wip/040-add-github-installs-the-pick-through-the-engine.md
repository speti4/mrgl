---
size: S
origin: done/003-github-release-source-with-release-json.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 040 — `add github:<owner>/<repo>` installs the pick through the engine

## Context

`mrgl add github:<owner>/<repo> --flavor <id>` without `--dry-run` downloads the picked asset (003) to
the flavor's staging folder and hands it to the engine's one install call (004) with the pick's source
and version as the metadata; the pick's version (the tag without a leading `v`) is what the sidecar
records, the TOC's `## Version` only a fallback. The "not implemented yet" answer goes away. The two
halves are on `main`; the first M1 command a user types is this one.
Done when: `add github:Questie/Questie --flavor forever` against the stub installs the fixture asset into
a temporary AddOns folder, the sidecar names `github:Questie/Questie` and `1.2.3`, and a second run
reports it as installed already.

Decided 2026-10-21: the download goes through the source's `Http` trait so the end-to-end test runs on
the folder stub and fixture zips, never the network; the zip lands under the flavor's staging folder and
is removed once the install call returns.

## Docs that move

- `README.md`: the usage line `mrgl add github:<owner>/<repo> --flavor forever` as an install, not a
  dry run only.
- `docs/development.md` ("Layout"): the add path from the pick to the engine.
- `crates/mrgl-cli/src/main.rs` module doc: `add github:` installs.

## Not in this change

- `update` and `update --all` (006); pin, ignore and the pre-release channel (021); the release notes
  line (030); the running-client warning on install (038).

## Steps

1. [ ] The download of the pick's asset through the source's `Http` into the flavor's staging folder
2. [ ] The engine's install call with the pick's source and version as the metadata; the sidecar entry
3. [ ] A second run reports the addon as installed already at that version (no download, no swap)
4. [ ] `mrgl-cli`: the `add github:` path without `--dry-run`, the voice lines, `--json`, end-to-end
       tests against the folder stub and the fixture zips
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-21 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
