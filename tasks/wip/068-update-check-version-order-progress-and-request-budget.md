---
size: M
origin: done/055-one-core-helper-for-resolve-download-and-install.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 068 — The update check: version order, progress, and a request budget without a token

## Context

`update::check` reports "installed is newer than the current release" as its own state instead of an update
(a deleted release, a channel switch); it reports progress per addon through a callback the CLI and the
window (010) share; and it stops with a clear line before a large flavor exhausts the 60 unauthenticated
requests an hour (042's reset time in the line). Today any other version counts as an update, a long check
is silent, and a long list without a token hits the rate limit part way.
Done when: fixture sidecars cover the newer-installed state, the progress lines, and a stubbed rate limit.

Decided 2026-10-22: its own branch; the progress callback is the handover to 010, which starts from this
branch's tip.

## Docs that move

- `docs/development.md`: the update states (the newer-installed one), the progress line, and the request
  budget without a token.
- `README.md` ("What it will do"), if the update check's lines are quoted there.

## Not in this change

- The window's update button and progress (010); a token prompt or store (the token comes from the
  environment as 042 reads it); the other sources (056).

## Steps

1. [ ] `update.rs`: the version order (the release list's order where the installed version is a known tag,
       else a lenient numeric compare) and a new `UpdateState` for an installed version newer than the pick,
       with its own voice line
2. [ ] A progress callback (`FnMut(Progress)` or a small trait) called once per addon, before and after its
       check; the CLI prints one line per addon through the voice, `--json` unchanged
3. [ ] A request budget on the GitHub source without a token: the requests of this run are counted, the
       check stops before the limit with the reset time of 042 in one line, the remaining addons are reported
       as not checked
4. [ ] Tests: fixture sidecars for the newer-installed state, the progress lines, and a stubbed rate limit
       (the stub answers the `x-ratelimit-*` headers)
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

## Deviations from the plan

- none yet

## Verification

- (dated, concrete: what was run, what it showed)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
