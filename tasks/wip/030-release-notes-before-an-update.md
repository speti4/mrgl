---
size: S
origin: session 2026-10-21
created: 2026-10-21
started: 2026-10-21
plan: none
milestone: M1 Core + CLI
---

# 030 — Release notes before an update: the GitHub release body in one line

## Context

The first line of the GitHub release body in `update --dry-run` and in the window's row (010), with the
release page's link; the body comes with the release list the source (003) already fetches. Knowing
what an update changes before taking it, and a link to the rest when one line is not enough.
Done when: a fixture release with a body shows its first line and the link, and an empty body shows
nothing.

Decided 2026-10-21: the pick carries the notes (`notes`: the first non-empty line of the release body,
Markdown markers and trailing whitespace stripped, cut at a documented length; `page`: the release's
`html_url`), both optional; `add --dry-run` shows them now, `update --dry-run` (006) and the window's
row (010) take them from the same pick when they land; the release list cache (003) keeps the body, so
nothing is fetched twice.

## Docs that move

- `docs/development.md` ("Layout"): the two fields on the pick and the first-line rule.
- `README.md`: the notes line of `add --dry-run`.

## Not in this change

- `update --dry-run` itself (006); the window's row (010); the full body or a pager.

## Steps

1. [ ] The release type reads `body` and `html_url` from the list; the pick gets `notes` and `page` with
       the first-line rule and its unit tests (empty body, a body of headings only, a long line)
2. [ ] `mrgl-cli`: `add --dry-run` shows the line and the link through the voice and in `--json`
3. [ ] Fixture: a release list with a body and one without, under the folder stub's fixtures; the
       end-to-end test for both
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

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
