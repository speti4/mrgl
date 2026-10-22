---
size: S
origin: done/009-tauri-shell-with-the-flavor-list.md
created: 2026-10-22
started: 2026-10-22
plan: none
milestone: M2 Window MVP
---

# 069 — The doctor report redacts a profile path written with forward slashes

## Context

The doctor's redaction of the user's profile folder (024) also matches the path spelled with `/` and in any
case, so "copy diagnostics" and `mrgl doctor` never carry the user name. A value read back from a config or
a web view can use forward slashes, and then the profile path lands in a public bug report.
Done when: the redaction test covers backslash, forward slash and mixed case, all printed as `~`.

Decided 2026-10-22: first of the chain 069 → 070 on one branch.

## Docs that move

- `docs/development.md`: the doctor's redaction rule, where it is stated.

## Not in this change

- Redacting anything other than the profile folder (an account name in a `WTF` path is the backup's and
  the sets' concern, not the doctor's); the window's diagnostics text beyond what the core redacts.

## Steps

1. [ ] `doctor.rs`: `Redactor` matches the long and the 8.3 short form of the profile folder with `\` or
       `/` as the separator, in any case, with the same "separator or end follows" rule
2. [ ] Tests: backslash, forward slash and mixed case, the short form too, all printed as `~`; a path that
       only starts with the profile's name is untouched
3. [ ] Docs under *Docs that move*; every check of CONTRIBUTING.md green; close

## Log

- 2026-10-22 — promoted; runs as a task session in its own worktree (docs/maintainer-sessions.md), first
  of the chain 069 → 070

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
