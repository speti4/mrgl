---
status: ready
size: S
origin: wip/001-project-scaffolding.md
created: 2026-10-21
milestone: M1 Core + CLI
---

# GitHub repository settings after the first push: rulesets, CodeQL, private reporting, security updates

What: the settings `docs/development.md` ("CI and releases") describes and no file carries: the ruleset on
`main` (pull request with squash merge only, the required checks `test`, `lint`, `msrv`, `deny`,
`workflow-lint` and `commit-rules`, linear history, no force push or deletion), the `v*` tag ruleset, CodeQL
default setup, squash merge as the only merge method with the pull request title and description as the
default message, private vulnerability reporting on, Dependabot security updates on.
Why: the git workflow and the security policy assume them; until the ruleset exists nothing stops a direct
push to `main`, and the SECURITY.md and code of conduct links 404 without private reporting.
Done when: every setting is on, a direct push to `main` is refused, and the advisory form link opens.
