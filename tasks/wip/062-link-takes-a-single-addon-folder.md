---
size: S
origin: done/005-dev-addon-linking-with-junctions.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M3 Toolbox
---

# 062 — `link` takes a single addon folder as well as a development folder

## Context

`mrgl link <folder>` given a folder that is itself an addon (a TOC named after it) links that one
folder instead of answering "no addon in the development folder"; `unlink` matches. A developer with one
addon keeps its repository as the addon folder, not under a parent.
Done when: a fixture addon folder links and unlinks on its own, and a development folder still links
its subfolders.

Decided 2026-10-22: first of the chain 062 → 073 on one branch, in wave 6.

## Docs that move

- `docs/development.md` and `README.md`: the `link` paragraph names the single addon folder shape.

## Not in this change

- A folder that is both an addon and a parent of addons (the addon shape wins, the subfolders are its own);
  the window's link action; the addon sets (073).

## Steps

1. [ ] `link.rs`: a folder with a TOC named after it is one addon to link; the sidecar entry records the
       source `link:<folder>` as for a development folder; `unlink` removes exactly that junction
2. [ ] CLI `link` and `unlink` report the one addon; the messages through the voice
3. [ ] Tests in `tests/link.rs` and `tests/cli.rs`: a fixture addon folder links and unlinks on its own, a
       development folder still links its subfolders
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 062 → 073

## Deviations from the plan

- none yet

## Verification

- <dated, concrete: what was run, what it showed>

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
