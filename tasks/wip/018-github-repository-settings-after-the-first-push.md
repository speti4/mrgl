---
size: S
origin: wip/001-project-scaffolding.md
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
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

1. [ ] Ruleset on `main` through `gh api repos/speti4/mrgl/rulesets`: pull request required, squash merge
       the only allowed method, the six required status checks, linear history, no force push, no deletion
2. [ ] Ruleset on `v*` tags: no update, no deletion
3. [ ] Repository settings through `gh api -X PATCH repos/speti4/mrgl`: squash merge only, the pull request
       title and body as the default squash message, delete the branch on merge
4. [ ] CodeQL default setup, private vulnerability reporting on, Dependabot security updates on
5. [ ] Verify: the rulesets listed, a direct push to `main` refused, the advisory form link opens

## Log

- 2026-10-21 — promoted; the state before the work recorded in Context

## Deviations from the plan

- none yet

## Verification

- pending

## Closing checklist

- [ ] every follow-up is a `backlog/` file (`/tasks add`) or deleted with a reason in the Log
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green, README index regenerated
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
