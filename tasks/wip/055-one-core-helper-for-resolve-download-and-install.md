---
size: M
origin: done/015-list-export-import-and-interface-warning.md
created: 2026-10-21
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
changelog: housekeeping
---

# 055 — One install path: a core helper for resolve, download and install, the link refusal in the commit

## Context

The resolve, download and engine-install steps that `add github:` (040, `Engine::install_pick`) and
`import` (015, `list_file::import`) each wrote fold into one core function both call, with one staging
folder rule and one already-installed rule, that resolves a managed addon on its own channel (021,
`Target::with_channel`) instead of the stable default; the refusal to install over a linked addon or a
foreign junction (005, `refuse_links`) moves from `Engine::install` into `Engine::commit`, so every path
that stages and commits is covered; and the CLI's install summary line and JSON block become one helper.
The one path that writes the user's folder was written twice and is about to be written a third time
(`update --all`, 006); a caller that forgets the link check overwrites a junction into a development folder.
Done when: one core function with three callers (`add github:`, `import`, `update`), the check once in
`commit`, one summary helper, and the CLI tests of the existing paths unchanged.

Decided 2026-10-22: this task is the first of the chain 055 → 006 → 057 on one branch; `update --all`
(006) is the third caller and lands in the same pull request.

## Docs that move

- `docs/development.md` ("Layout"): the one install helper named as the path every source-backed install
  takes, and where the link refusal lives.

## Not in this change

- The update check and `update --all` themselves (006, next in the chain); `import --exact` (057).
- Any change to the staging, swap or revert rules of the engine (004), to the marks (021) or to the link
  module (005) beyond moving the one refusal.

## Steps

1. [ ] The core helper (resolve on the entry's channel, download, stage, commit) with `add github:` and
       `import` as its callers; the duplicated code in `Engine::install_pick` and `list_file::import` removed
2. [ ] `refuse_links` called once in `Engine::commit`; the tests of 005 (install over a linked addon, over a
       foreign junction) pass through every path
3. [ ] One CLI summary helper (the plain lines and the JSON block) for every install path
4. [ ] The existing CLI tests of `add github:` and `import` unchanged and green
5. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 055 → 006 → 057

## Deviations from the plan

- none yet

## Verification

- (dated, concrete, at close)

## Closing checklist

- [ ] every follow-up is listed in the pull request under `## Follow-ups`, the main session files them
- [ ] every doc under *Docs that move* changed in this change
- [ ] Verification written, dated
- [ ] `tasks_check` green (the README index is the main session's)
- [ ] moved to `done/` with `closed: YYYY-MM-DD`
