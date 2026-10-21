---
size: S
origin: wip/001-project-scaffolding.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
closed: 2026-10-21
---

# 018 — GitHub repository settings after the first push: rulesets, CodeQL, private reporting, security updates

## Context

The settings `docs/development.md` ("CI and releases") describes and no file carries: the ruleset on `main`
(pull request with squash merge only, the required checks `test`, `lint`, `msrv`, `deny`, `workflow-lint`
and `commit-rules`, linear history, no force push or deletion), the `v*` tag ruleset, CodeQL default setup,
squash merge as the only merge method with the pull request title and description as the default message,
private vulnerability reporting on, Dependabot security updates on. The git workflow and the security policy
assume them; until the ruleset exists nothing stops a direct push to `main`, and the SECURITY.md and code of
conduct links 404 without private reporting. Checked 2026-10-21 before the work: no ruleset, all three merge
methods allowed, the squash message taken from the commit list, CodeQL not configured, private reporting
off, Dependabot security updates disabled. Done when every setting is on, a direct push to `main` is refused,
and the advisory form link opens.

## Docs that move

- none: `docs/development.md` already describes every setting; this task makes the repository match it.

## Not in this change

- The task-to-issue mirror workflow and its backfill (016).
- Branch rules for anything but `main` and the `v*` tags.
- Secret scanning settings beyond what GitHub enables by default on a public repo.

## Steps

1. [x] Ruleset on `main` through `gh api repos/speti4/mrgl/rulesets`: pull request required, squash merge
       the only allowed method, the six required status checks, linear history, no force push, no deletion
2. [x] Ruleset on `v*` tags: no update, no deletion
3. [x] Repository settings through `gh api -X PATCH repos/speti4/mrgl`: squash merge only, the pull request
       title and body as the default squash message, delete the branch on merge
4. [x] CodeQL default setup, private vulnerability reporting on, Dependabot security updates on
5. [x] Verify: the rulesets listed, a direct push to `main` refused, the advisory form link opens

## Log

- 2026-10-21 — promoted; the state before the work recorded in Context
- 2026-10-21 — every setting made through the REST API (`gh api`), from two JSON ruleset bodies: the `main`
  ruleset (`deletion`, `non_fast_forward`, `required_linear_history`, `pull_request` with `squash` as the
  only merge method, `required_status_checks` for the six jobs of the GitHub Actions app, no bypass actors),
  the `release tags` ruleset on `refs/tags/v*` (`deletion`, `non_fast_forward`, `update`), the repository
  flags (`allow_squash_merge` only, `PR_TITLE` + `PR_BODY` as the squash message, `delete_branch_on_merge`),
  private vulnerability reporting, Dependabot alerts with security updates, CodeQL default setup with the
  `default` query suite

## Deviations from the plan

- none

## Verification

- 2026-10-21 — `gh api repos/speti4/mrgl/rulesets` lists `main` (branch, active) and `release tags` (tag,
  active); `gh api repos/speti4/mrgl` shows merge commits and rebase merges off, squash on with
  `PR_TITLE` / `PR_BODY`, branch deletion on merge on, Dependabot security updates `enabled`;
  `private-vulnerability-reporting` returns `enabled: true`; `code-scanning/default-setup` returns
  `configured` with the `default` suite and its first `CodeQL Setup` workflow run started.
- 2026-10-21 — a direct push of an empty commit to `main` from a detached HEAD was refused with `GH013`
  ("Changes must be made through a pull request", "6 of 6 required status checks are expected"); `main`
  on GitHub is unchanged.
- 2026-10-21 — the advisory form URL of SECURITY.md answers with a redirect to login (302), the advisories
  page with 200; neither is a 404 any more.

## Closing checklist

- [x] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
- [x] every doc under *Docs that move* changed in this change
- [x] Verification written, dated
- [x] `tasks_check` green, README index regenerated
- [x] moved to `done/` with `closed: YYYY-MM-DD`
