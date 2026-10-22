---
size: S
origin: done/079-window-and-exe-polish-after-the-first-release.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 082 — An install and an update report the whole interface check: out of date next to the mismatch, and `update` says it at all

## Context

`add <file.zip>`, `import` and `update` warn after an install when the addon's TOC is behind the build
(`out of date`, 076) as they warn about a mismatch, in the text and in the JSON (`interface_out_of_date`
next to `interface_mismatch`); and `mrgl update` prints the interface warning it computes in
`install_summary` and carries it in its JSON document, which it drops today.
Why: after a client update every updated addon with the previous interface number shows `Out of date` in the
client while the install summary says nothing, and `update` says nothing about a mismatch either.
Done when: the CLI tests of `add`, `import` and `update` over the `stale-interface` and a behind-the-build
fixture zip see both warnings in the text and the JSON.

Decided 2026-10-22: runs as the first task of a chain with 083 and 087 on one branch, since all three change
the command line's install summary and `update` output.

## Docs that move

- `docs/development.md`: the install summary's warnings (both interface states) and the `update` JSON document
- `README.md`: the `out of date` mark where the install warnings are described, if the text names them

## Not in this change

- The window's rows (081) and the window's update answer, which already carries the core's checks (079)
- A reinstall's own wording (083) and the last-check file (087)

## Steps

1. [ ] The core's installed checks (`Engine::check_installed`, `InstalledChecks::lines`) carry the out-of-date
       state next to the mismatch, with one warning line each through the voice
2. [ ] `mrgl-cli/src/main.rs`: `add <file.zip>`, `import` and `update` print both warnings and carry
       `interface_out_of_date` next to `interface_mismatch` in their JSON
3. [ ] Tests in `crates/mrgl-cli/tests/cli.rs` over the `stale-interface` zip and a behind-the-build fixture
       zip: both warnings in the text and the JSON, for `add`, `import` and `update`
4. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md)

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
