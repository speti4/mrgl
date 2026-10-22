---
size: S
origin: done/045-pre-launch-manual-round-on-the-real-install.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M1 Core + CLI
---

# 075 — The CLI's output survives a closed pipe

## Context

Every command's stdout and stderr writes go through one helper that treats a closed pipe (`BrokenPipe`,
os error 232 on Windows) as the end of the run: the command exits quietly with 0 instead of panicking.
`mrgl list --json | head -c 200` on the real install panicked with `failed printing to stdout`, which
breaks every shell pipeline that reads only the start of the output.
Done when: a test pipes `list --json` into a reader that closes early and sees exit 0 with no panic text, and
the plain and JSON outputs are unchanged otherwise.

Decided 2026-10-22: second of the chain 077 → 075 on one branch, in wave 6.

## Docs that move

- `docs/development.md`: the command line's output rule for a closed pipe, next to the voice paragraph.

## Not in this change

- The window (no pipe); a change to any message text.

## Steps

1. [ ] `mrgl-cli/src/main.rs`: the `Reporter` methods and the remaining raw `println!`/`eprintln!` sites write
       through one helper that maps a closed pipe to a quiet end of the run (exit 0)
2. [ ] Test: `list --json` piped into a reader that closes early exits 0 with no panic text; the plain and
       JSON outputs of the existing tests unchanged
3. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), second
  of the chain 077 → 075

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
